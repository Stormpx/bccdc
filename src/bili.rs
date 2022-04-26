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
   ttl: u64,
   data: Value,
}

impl BilibiliResult {
    fn data(&self)-> Result<&Value,Box<dyn Error>>{
        if self.code != 0{
            return Err(Box::from(self.message.to_string()));
        }
        Ok(&self.data)
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
    
}
