use std::fs;
use url::{Url};
use std::path::Path;
use std::error::Error;
use serde_json::{Value,Deserializer};
use crate::cc::{CcSubtitle,Line};

use crate::bili;

static ERR_MSG: &str = "invalid bcc file";

pub enum Page{
    All,
    Single(u32),
    Range(u32,u32),
}

impl Page {
    fn test(&self,p: &u32)->bool{
        match self{
            Page::All => true,
            Page::Single(i) => i==p,
            Page::Range(s,e) => s<=p&&e>=p,
        }
        
    }
}

pub struct VideoPage{
    pub p: u32,
    pub subtitles: Vec<CcSubtitle>,
}



pub fn json_to_subtitle(name: &str,content: &str)-> Result<CcSubtitle,Box<dyn Error>> {
    let r = serde_json::from_str(content);
    if let Err(_) = r{
        return Err(ERR_MSG.into());
    }
    let v: Value = r?;
    
    let body: &Value = &v["body"];
    
    let objs: &Vec<Value>=body.as_array().ok_or(ERR_MSG)?;

    let lines: Result<Vec<Line>,Box<dyn Error>>=objs.iter().map(|obj| {
        let content= obj["content"].as_str().ok_or::<Box<dyn Error>>(ERR_MSG.into())?.to_string();
        let start = obj["from"].as_f64().ok_or::<Box<dyn Error>>(ERR_MSG.into())?;
        let end = obj["to"].as_f64().ok_or::<Box<dyn Error>>(ERR_MSG.into())?;
        Ok(Line{content,start,end})
    }).collect();
    
    Ok(CcSubtitle{
        name: name.to_string(),
        lan: None,
        lan_doc: None,
        lines: lines?,
    })

}

pub fn lookup_file(path: &Path)-> Result<CcSubtitle,Box<dyn Error>> {

    let name = path.file_stem().unwrap().to_str().unwrap();
    let content=fs::read_to_string(path)?;
    json_to_subtitle(name,&content)

}

pub fn lookup_cc_api(url: &Url) -> Result<CcSubtitle,Box<dyn Error>>{
    let mut file_name=url.path_segments().ok_or::<Box<dyn Error>>("url path required".into())?
        .last().ok_or::<Box<dyn Error>>("invalid url".into())?;

    if let Some(p) = file_name.rfind("."){
        file_name= &file_name[0..p];
    }

    let content = bili::simple_http_get(url,&vec![])?;

    json_to_subtitle(file_name,&content)

}

fn find_id(ep_html: &str)-> Option<(String,u64)>{

    let flag = "window.__INITIAL_STATE__=";
    
    let index=ep_html.find(flag)?;
    let data = &ep_html[index+flag.len()..];

    
    let stream = Deserializer::from_str(data).into_iter::<Value>();

    for value in stream {
        if let Ok(json) = value{
            // println!("{}",json.to_string());
            if let  Value::Object(ep_info) = &json["epInfo"]{
                let bvid= &ep_info["bvid"].as_str();
                let cid = &ep_info["cid"].as_u64();
                if bvid.is_some() && cid.is_some(){
                   return Some((bvid.unwrap().to_string(),cid.unwrap())); 
                }
            }else if let Value::Object(video_data) = &json["videoData"]{
                let bvid= &video_data["bvid"].as_str();
                let cid = &video_data["cid"].as_u64();
                
                if bvid.is_some() && cid.is_some(){
                   return Some((bvid.unwrap().to_string(),cid.unwrap())); 
                }
            }
        }
    }

    None 

}

fn get_subtitles(bvid: &str,cid: u64,page: u32)-> Result<Vec<CcSubtitle>,Box<dyn Error>>{
    let list= bili::get_subtitle_list(&bvid,&cid)?;

    let mut result = Vec::new();
    for info in list {
        if let Some(url) = info.url(){
            match lookup_cc_api(&url){
                Ok(mut cc)=> {
                    cc.lan = Some(info.lan);
                    cc.lan_doc = Some(info.lan_doc);
                    result.push(cc)
                },
                Err(e)=> {
                    eprintln!("fail to download {}-p{} subtitle: {} cause: {}",bvid,page,info.lan_doc,e);
                }
            }
        }
    }
    Ok(result)

}

pub fn lookup_ep_id(id: &str)-> Result<Vec<CcSubtitle>,Box<dyn Error>>{
    let content=bili::get_ep_html(id)?;

    let (bvid,cid) = find_id(&content).ok_or::<Box<dyn Error>>(format!("unable find bvid and cid by {}",id).into())?;

    get_subtitles(&bvid,cid,1)
    
}

pub fn lookup_video_id(id: &str,interested: Vec<Page>)-> Result<Vec<VideoPage>,Box<dyn Error>>{
    let mut bvid = id.to_string();
    if bvid.starts_with("av"){
        let aid = id[2..].parse::<u64>()?;
        bvid = bili::av_to_bv(&aid);
    }

    let page_list = bili::get_page_list(&bvid.trim())?;

    let vsubs : Vec<VideoPage>= page_list.iter()
        .filter(|page| interested.iter().any(|range| range.test(&page.page)))
        .map(|page| { 
            
            let r = get_subtitles(&bvid,page.cid,page.page)
                .map(|subs| VideoPage{p: page.page,subtitles: subs});

            if let Err(ref e) = r{
                eprintln!("fail to get subtitle list. cause: {}",e);
            }
            r

        })
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap())
        .collect();

    Ok(vsubs)    
}

pub fn lookup_media_id(id:&str,interested: Vec<Page>)-> Result<Vec<VideoPage>,Box<dyn Error>>{
    let season_id = bili::get_season_id(id)?;
    
    let episodes = bili::get_season_episodes(&season_id)?;

    let r = episodes.iter().enumerate()
        .filter(|(index,_ep)| interested.iter().any(|range| range.test(&((index+1) as u32))))
        .map(|(index,ep)| {
            let bvid = bili::av_to_bv(&ep.aid);
            let p = (index+1) as u32;
            let r = get_subtitles(&bvid,ep.cid,p)
                .map(|subs| VideoPage{p:p,subtitles: subs});
            if let Err(ref e) = r{
                eprintln!("fail to get subtitle. cause: {}",e);
            }
            r
        })
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap())
        .collect();
    Ok(r)
}

#[cfg(test)]
mod tests{
    use tempfile::NamedTempFile;
    use std::fs;
    use crate::lookup;
    #[test]
    fn lookup_file_test(){
        let json= "{\"body\":[{\"content\":\"花蕾 石屑 又一輪循環\",\"from\":1341.19,\"location\":2,\"to\":1343.27}]}";
        let tempfile=NamedTempFile::new().unwrap();
        fs::write(&tempfile,json).expect("fail to write tempfile");
        let temppath= tempfile.path();
        let subtitle = crate::lookup::lookup_file(temppath).unwrap(); 
        assert_eq!(temppath.file_name().unwrap().to_str(),Some(&subtitle.name[..]));
        let line= &subtitle.lines[0];
        assert_eq!("花蕾 石屑 又一輪循環",&line.content);
        assert_eq!(1341.19,line.start);
        assert_eq!(1343.27,line.end);
        tempfile.close().expect("");
    }

    #[test]
    fn find_id_test(){
        let content = "</script><script>window.__INITIAL_STATE__={\"epInfo\":{\"aid\":937924663,\"badge\":\"会员\",\"badge_info\":{\"bg_color\":\"#FB7299\",\"bg_color_night\":\"#BB5B76\",\"text\":\"会员\"},\"badge_type\":0,\"bvid\":\"BV1zT4y1v7kC\",\"cid\":569612278,\"cover\":\"\\u002F\\u002Fi0.hdslb.com\\u002Fbfs\\u002Farchive\\u002Ff5e5f123aef7399156a6fe74d4cb7aaf97604a20.png\",\"dimension\":{\"height\":1080,\"rotate\":0,\"width\":1920},\"duration\":1421000,\"from\":\"bangumi\",\"id\":475899,\"is_view_hide\":false,\"link\":\"https:\\u002F\\u002Fwww.bilibili.com\\u002Fbangumi\\u002Fplay\\u002Fep475899\",\"long_title\":\"孔明 施展計謀\",\"pub_time\":1649340000,\"pv\":0,\"release_date\":\"\",\"rights\":{\"allow_demand\":0,\"allow_dm\":1,\"allow_download\":0,\"area_limit\":0},\"share_copy\":\"《派對咖孔明（僅限港澳台地區）》第2话 孔明 施展計謀\",\"share_url\":\"https:\\u002F\\u002Fwww.bilibili.com\\u002Fbangumi\\u002Fplay\\u002Fep475899\",\"short_link\":\"https:\\u002F\\u002Fb23.tv\\u002Fep475899\",\"status\":13,\"subtitle\":\"已观看18万次\",\"title\":\"2\",\"vid\":\"\",\"loaded\":true,\"badgeType\":0,\"badgeColor\":\"#FB7299\",\"epStatus\":13,\"titleFormat\":\"第2话\",\"longTitle\":\"孔明 施展計謀\",\"sectionType\":0,\"releaseDate\":\"\",\"skip\":{},\"stat\":{},\"orderSectionIds\":[],\"hasNext\":false,\"hasSkip\":false,\"i\":1}};(function(){vars;(s=document.currentScript||document.scripts[document.scripts.length-1]).parentNode.removeChild(s);}());</script><scripttype=\"text/javascript\">";

        assert_eq!(lookup::find_id(content),Some((String::from("BV1zT4y1v7kC"),569612278)));
    }

    #[test]
    fn find_id_test1(){
        let content = "</script><script>window.__INITIAL_STATE__={\"aid\":1703355805,\"bvid\":\"BV1mT42127CQ\",\"p\":1,\"episode\":\"\",\"videoData\":{\"bvid\":\"BV1mT42127CQ\",\"aid\":1703355805,\"videos\":1,\"tid\":201,\"tname\":\"科学科普\",\"copyright\":2,\"pic\":\"http://i1.hdslb.com/bfs/archive/4786b7bc4a268ea0bb31bf66be6aa6eee1a04c33.png\",\"title\":\"【地理】直布罗陀海峡形成与地中海的滔天洪水\",\"pubdate\":1713668408,\"ctime\":1713622289,\"desc\":\"530万年前，一次大冰期导致了海平面下降，地中海与大西洋沟通消失，75%的地中海蒸发演变为沙漠。当冰期结束海平面再次上升,形成了直布罗陀瀑布,从最初涓流细流到冲刷出了一道越来越深的缺口用了数千年，最后，当直布罗陀变成了亚马逊河流量的1000倍的巨大瀑布时，仅用几个月将地中海90%的面积填满。【地址】https://youtu.be/_RSPgIcnRN0\",\"desc_v2\":[{\"raw_text\":\"530万年前，一次大冰期导致了海平面下降，地中海与大西洋沟通消失，75%的地中海蒸发演变为沙漠。当冰期结束海平面再次上升,形成了直布罗陀瀑布,从最初涓流细流到冲刷出了一道越来越深的缺口用了数千年，最后，当直布罗陀变成了亚马逊河流量的1000倍的巨大瀑布时，仅用几个月将地中海90%的面积填满。【地址】https://youtu.be/_RSPgIcnRN0\",\"type\":1,\"biz_id\":0}],\"state\":0,\"duration\":794,\"argue_info\":{\"argue_msg\":\"\",\"argue_type\":0,\"argue_link\":\"\"},\"dynamic\":\"\",\"cid\":1514080023,\"dimension\":{\"width\":1280,\"height\":720,\"rotate\":0},\"season_id\":2545199,\"premiere\":null,\"teenage_mode\":0,\"is_chargeable_season\":false,\"is_story\":false,\"is_upower_exclusive\":false,\"is_upower_play\":false,\"is_upower_preview\":false,\"enable_vt\":0,\"vt_display\":\"\",\"no_cache\":false,\"is_season_display\":true,\"user_garb\":{\"url_image_ani_cut\":\"\"},\"honor_reply\":{},\"like_icon\":\"\",\"need_jump_bv\":false,\"disable_show_up_info\":false,\"is_story_play\":1},\"isCollection\":0,\"sectionsInfo\":{\"id\":2545199,\"title\":\"地理* 海陆变迁与板块构造\",\"cover\":\"https://s1.hdslb.com/bfs/templar/york-static/viedeo_material_default.png\",\"mid\":224888695,\"intro\":\"\",\"sign_state\":0,\"attribute\":140,\"sections\":[],\"stat\":{\"season_id\":2545199,\"view\":1506470,\"danmaku\":8664,\"reply\":5305,\"fav\":25040,\"coin\":7379,\"share\":3511,\"now_rank\":0,\"his_rank\":0,\"like\":38670,\"vt\":0,\"vv\":0},\"ep_count\":42,\"season_type\":1,\"is_pay_season\":false,\"enable_vt\":0},\"playedSectionId\":[],\"sections\":[],\"staffData\":[],\"isClient\":false,\"error\":{},\"player\":null,\"playurl\":{},\"user\":{},\"cidMap\":{\"1703355805\":{\"aid\":1703355805,\"bvid\":\"BV1mT42127CQ\",\"cids\":{\"1\":1514080023}},\"BV1mT42127CQ\":{\"aid\":1703355805,\"bvid\":\"BV1mT42127CQ\",\"cids\":{\"1\":1514080023}}},\"isRecAutoPlay\":false,\"continuousPlay\":true,\"autoPlayNextVideo\":null,\"elecFullInfo\":{},\"emergencyBan\":{\"no_like\":false,\"no_coin\":false,\"no_fav\":false,\"no_share\":false},\"isModern\":true,\"playerReloadOrigin\":\"\",\"queryTags\":[],\"nanoTheme\":{\"bpx-primary-color\":\"var(--brand_blue)\",\"bpx-fn-color\":\"var(--brand_blue)\",\"bpx-fn-hover-color\":\"var(--brand_blue)\",\"bpx-box-shadow\":\"var(--bg3)\",\"bpx-dmsend-switch-icon\":\"var(--text2)\",\"bpx-dmsend-hint-icon\":\"var(--graph_medium)\",\"bpx-aux-header-icon\":\"var(--graph_icon)\",\"bpx-aux-float-icon\":\"var(--graph_icon)\",\"bpx-aux-block-icon\":\"var(--text3)\",\"bpx-dmsend-info-font\":\"var(--text2)\",\"bpx-dmsend-input-font\":\"var(--text1)\",\"bpx-dmsend-hint-font\":\"var(--text3)\",\"bpx-aux-header-font\":\"var(--text1)\",\"bpx-aux-footer-font\":\"var(--text2)\",\"bpx-aux-footer-font-hover\":\"var(--text1)\",\"bpx-aux-content-font1\":\"var(--text1)\",\"bpx-aux-content-font2\":\"var(--text2)\",\"bpx-aux-content-font3\":\"var(--text2)\",\"bpx-aux-content-font4\":\"var(--text3)\",\"bpx-aux-content-font5\":\"var(--text3)\",\"bpx-dmsend-main-bg\":\"var(--bg1)\",\"bpx-dmsend-input-bg\":\"var(--bg3)\",\"bpx-aux-header-bg\":\"var(--graph_bg_regular)\",\"bpx-aux-footer-bg\":\"var(--graph_bg_regular)\",\"bpx-aux-content-bg\":\"var(--bg1)\",\"bpx-aux-button-bg\":\"var(--bg3)\",\"bpx-aux-button-disabled-bg\":\"var(--graph_bg_thin)\",\"bpx-aux-float-bg\":\"var(--bg1_float)\",\"bpx-aux-float-hover-bg\":\"var(--graph_medium)\",\"bpx-aux-cover-bg\":\"var(--graph_weak)\",\"bpx-dmsend-border\":\"var(--bg3)\",\"bpx-aux-float-border\":\"var(--line_light)\",\"bpx-aux-line-border\":\"var(--line_regular)\",\"bpx-aux-input-border\":\"var(--line_regular)\"},\"enable_vt\":0,\"defaultWbiKey\":{\"wbiImgKey\":\"2590160e9f5142d4a501feda0490f3bd\",\"wbiSubKey\":\"34ba9c5c4a824b368e9c053be34016bd\"},\"bmpDefDomain\":\"\",\"loadingRcmdTabData\":false,\"rcmdTabData\":{\"tab_name\":\"全部\",\"archives\":[],\"has_more\":false},\"rcmdTabNames\":[\"科学\",\"科普\",\"灾难\",\"地理\",\"古地理\",\"地质巨变\",\"洪水\",\"地中海\"],\"currentRcmdTab\":{\"tab_name\":\"全部\",\"tab_order\":0,\"tab_type\":1}}";

        assert_eq!(lookup::find_id(content),Some((String::from("BV1mT42127CQ"),1514080023)));
    }
}

