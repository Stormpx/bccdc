use url::{Url};
use reqwest::header;
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use once_cell::sync::OnceCell; 
use once_cell::sync::Lazy;

static EP_URL: Lazy<Url> = Lazy::new(||  Url::parse("https://www.bilibili.com/bangumi/play/").unwrap());
static PLAYER_URL: Lazy<Url> = Lazy::new(|| Url::parse("https://api.bilibili.com/x/player/v2").unwrap());
static PAGE_LIST_URL: Lazy<Url> = Lazy::new(|| Url::parse("https://api.bilibili.com/x/player/pagelist").unwrap());
static SEASON_SECTION_URL: Lazy<Url> = Lazy::new(|| Url::parse("https://api.bilibili.com/pgc/web/season/section").unwrap());

static HTTP_CLIENT: OnceCell<reqwest::blocking::Client> = OnceCell::new();

pub fn init_client(proxy: Option<String>) -> Result<(),Box<dyn Error>>{
    let mut  builder = reqwest::blocking::Client::builder()
        .gzip(true);
        
    if let Some(proxy)= proxy{
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }

    let client = builder.build()?;
    HTTP_CLIENT.set(client).unwrap();
    Ok(())
}

fn client()-> &'static reqwest::blocking::Client{
    HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .gzip(true)
            .build().unwrap()
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct BilibiliResult{
   code: i64,
   message: String,
   ttl: Option<u64>,
   #[serde(default)]
   data: Value,
   #[serde(default)]
   result: Value,

}

impl BilibiliResult {
    fn data(&self)-> Result<&Value,Box<dyn Error>>{
        if self.code != 0{
            return Err(Box::from(self.message.to_string()));
        }
        Ok(&self.data)
    }
    fn result(&self)-> Result<&Value,Box<dyn Error>>{
        if self.code != 0{
            return Err(Box::from(self.message.to_string()));
        }
        Ok(&self.result)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubtitleInfo{
   pub id : u64,
   pub lan: String,
   pub lan_doc: String,
   pub subtitle_url: String,
   pub r#type: u8,
}

impl SubtitleInfo {
    pub fn url(&self)-> Option<Url>{
       if self.subtitle_url.starts_with("http"){
            return Url::parse(&self.subtitle_url).ok();
       }
       Url::parse(&format!("https:{}",self.subtitle_url)).ok()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageInfo{
   pub cid: u64,
   pub page: u32,
   pub part: String,
   pub duration: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Episodes{
    pub id: u64,
    pub aid: u64,
    pub cid: u64,
    pub badge: Option<String>,
    pub badge_info: Option<Value>,
    pub badge_type: Option<u8>,
    pub cover: Option<String>,
    pub from: Option<String>,
    pub is_premiere: Option<u8>,
    pub long_title: Option<String>,
    pub share_url: Option<String>,
    pub status: Option<u8>,
    pub title: Option<String>,
    pub vid: Option<String>,

}


pub fn simple_http_get(url: &Url,query: &Vec<(&str,&str)> )-> Result<String,Box<dyn Error>>{

    let resp = client().get(url.as_str())
        .header(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header(header::ACCEPT_LANGUAGE,"en-US,en;q=0.5")
        .header(header::ACCEPT_ENCODING, "gzip")
        .header(header::USER_AGENT,"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/101.0")
        .query(query)
        .send()?;

     if resp.status().is_success() {
        return Ok(resp.text()?)
     } else{
        return Err(format!("request {} return {}",resp.url(),resp.status()).into());
     } 

}

pub fn get_ep_html(ep_id: &str)-> Result<String,Box<dyn Error>>{
    let url= EP_URL.join(ep_id)?;
    simple_http_get(&url,&vec![])
}

pub fn get_subtitle_list(bvid:&str,cid:u64)-> Result<Vec<SubtitleInfo>,Box<dyn Error>>{
    let content= simple_http_get(&PLAYER_URL,&vec![("bvid",bvid),("cid",&cid.to_string())])?;
    let result: BilibiliResult = serde_json::from_str(&content)?;

    let data = result.data()?; 
     
    let subtitle = &data["subtitle"];
    let subtitles = &subtitle["subtitles"];

    Ok(subtitles.as_array().ok_or::<Box<dyn Error>>("can't find subtitels".into())?
        .iter()
        .filter(|x|  x.is_object())
        .map(|i| SubtitleInfo::deserialize(i))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect())

}

pub fn get_page_list(bvid: &str)-> Result<Vec<PageInfo>,Box<dyn Error>> {
    let content= simple_http_get(&PAGE_LIST_URL,&vec![("bvid",bvid),("jsonp","jsonp")])?;
    
    let result: BilibiliResult = serde_json::from_str(&content)?;

    let data = result.data()?; 
    let page_list = Vec::<PageInfo>::deserialize(data)?; 

    Ok(page_list) 
}

pub fn get_season_episodes(season_id: &u64) -> Result<Vec<Episodes>,Box<dyn Error>> {
    let content= simple_http_get(&SEASON_SECTION_URL,&vec![("season_id",&season_id.to_string())])?;
    let result: BilibiliResult = serde_json::from_str(&content)?;
    
    let result = result.result()?;
    let main_section = result["main_section"].as_object().ok_or::<Box<dyn Error>>("main_section".into())?;
    let episodes =  &main_section["episodes"];

    let eps  = Vec::<Episodes>::deserialize(episodes)?; 
    Ok(eps)

}

#[cfg(test)]
mod tests{
    use crate::bili;
    #[test]
    fn get_subtitle_list_test(){
        let bvid = "BV1zT4y1v7kC";
        let cid = 569612278;
        
        let infos=bili::get_subtitle_list(bvid,cid).unwrap();

        let subtitle = &infos[0];

        assert_eq!(subtitle.id,932631245551156736);
        assert_eq!(subtitle.lan,"zh-Hant");
        assert_eq!(subtitle.lan_doc,"中文（繁体）");
        assert_eq!(subtitle.subtitle_url,"//i0.hdslb.com/bfs/subtitle/b7d807cb5df496ad1276e29637704c5f5dc80f43.json");

        println!("{:?}",infos);


    }
    #[test]
    fn get_page_list(){
            
        let bvid = "BV1zT4y1v7kC";

        let pages = bili::get_page_list(bvid).unwrap();
        
        let page= &pages[0];

        assert_eq!(page.cid,569612278);
        assert_eq!(page.page,1);
        assert_eq!(page.part, "PP02_Haishin_R.encoded");
        assert_eq!(page.duration,1421);
    }
    #[test]
    fn season_section(){
        let test_case: [(u64,u64);13] =  [
            (11931200,19695814),
            (209041759,19695821),
            (294034570,19695822),
            (591600094,19695823),
            (634103120,19695815),
            (379112581,19695824),
            (721602076,19695816),
            (464008075,19695825),
            (676602083,19695817),
            (764081645,19695818),
            (721608355,19695819),
            (719079280,19695813),
            (934101570,19695820),
        ];
        let season_id= 752;

        let eps = bili::get_season_episodes(&season_id).unwrap();

        for (i,ep) in eps.iter().enumerate(){
            assert_eq!(ep.aid,test_case[i].0);
            assert_eq!(ep.cid,test_case[i].1);
        }


    }
    
}
