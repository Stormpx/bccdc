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
            if let  Value::Object(ep_info) = &json["epInfo"]{
                let  bvid= &ep_info["bvid"].as_str();
                let cid = &ep_info["cid"].as_u64();
                if bvid.is_some() && cid.is_some(){
                   return Some((bvid.unwrap().to_string(),cid.unwrap())); 
                }
            }

        }
    }

    None 

}

fn get_subtitles(bvid: &str,cid: u64,page: u32)-> Result<Vec<CcSubtitle>,Box<dyn Error>>{
    let list= bili::get_subtitle_list(&bvid,cid)?;

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

    //println!("{}",content);
    let (bvid,cid) = find_id(&content).ok_or::<Box<dyn Error>>(format!("unable find bvid and cid by {}",id).into())?;

    get_subtitles(&bvid,cid,1)
    
}

pub fn lookup_video_id(id: &str,interested: Vec<Page>)-> Result<Vec<VideoPage>,Box<dyn Error>>{
    if id.starts_with("av"){
        
    }

    let page_list = bili::get_page_list(id)?;

    let vsubs : Vec<VideoPage>= page_list.iter()
        .filter(|page| interested.iter().any(|range| range.test(&page.page)))
        .map(|page| { 
            
            let r = get_subtitles(id,page.cid,page.page)
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
        tempfile.close();
    }

    #[test]
    fn find_id_test(){
        let content = "</script><script>window.__INITIAL_STATE__={\"epInfo\":{\"aid\":937924663,\"badge\":\"会员\",\"badge_info\":{\"bg_color\":\"#FB7299\",\"bg_color_night\":\"#BB5B76\",\"text\":\"会员\"},\"badge_type\":0,\"bvid\":\"BV1zT4y1v7kC\",\"cid\":569612278,\"cover\":\"\\u002F\\u002Fi0.hdslb.com\\u002Fbfs\\u002Farchive\\u002Ff5e5f123aef7399156a6fe74d4cb7aaf97604a20.png\",\"dimension\":{\"height\":1080,\"rotate\":0,\"width\":1920},\"duration\":1421000,\"from\":\"bangumi\",\"id\":475899,\"is_view_hide\":false,\"link\":\"https:\\u002F\\u002Fwww.bilibili.com\\u002Fbangumi\\u002Fplay\\u002Fep475899\",\"long_title\":\"孔明 施展計謀\",\"pub_time\":1649340000,\"pv\":0,\"release_date\":\"\",\"rights\":{\"allow_demand\":0,\"allow_dm\":1,\"allow_download\":0,\"area_limit\":0},\"share_copy\":\"《派對咖孔明（僅限港澳台地區）》第2话 孔明 施展計謀\",\"share_url\":\"https:\\u002F\\u002Fwww.bilibili.com\\u002Fbangumi\\u002Fplay\\u002Fep475899\",\"short_link\":\"https:\\u002F\\u002Fb23.tv\\u002Fep475899\",\"status\":13,\"subtitle\":\"已观看18万次\",\"title\":\"2\",\"vid\":\"\",\"loaded\":true,\"badgeType\":0,\"badgeColor\":\"#FB7299\",\"epStatus\":13,\"titleFormat\":\"第2话\",\"longTitle\":\"孔明 施展計謀\",\"sectionType\":0,\"releaseDate\":\"\",\"skip\":{},\"stat\":{},\"orderSectionIds\":[],\"hasNext\":false,\"hasSkip\":false,\"i\":1}};(function(){vars;(s=document.currentScript||document.scripts[document.scripts.length-1]).parentNode.removeChild(s);}());</script><scripttype=\"text/javascript\">";

        assert_eq!(lookup::find_id(content),Some((String::from("BV1zT4y1v7kC"),569612278)));
    }
}

