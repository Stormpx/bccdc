use url::{Url};
use reqwest::header;
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static!{
    static ref HTTP_CLIENT: reqwest::blocking::Client= reqwest::blocking::Client::builder()
        .gzip(true)
        .build().unwrap();
    static ref EP_URL: Url = Url::parse("https://www.bilibili.com/bangumi/play").unwrap();
    static ref PLAYER_URL:Url = Url::parse("https://api.bilibili.com/x/player/v2").unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
struct BilibiliResult{
   code: u64,
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
   id : u64,
   lan: String,
   lan_doc: String,
   subtitle_url: String,
   r#type: u8,
}

impl SubtitleInfo {
    pub fn url(&self)-> String{
       if self.subtitle_url.starts_with("http"){
            return self.subtitle_url.to_string();
       }
       format!("https:{}",self.subtitle_url)
    }
}

pub fn simple_http_get(url: &Url,query: &HashMap<&str,String> )-> Result<String,Box<dyn Error>>{

    Ok(HTTP_CLIENT.get(url.as_str())
        .header(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header(header::ACCEPT_LANGUAGE,"en-US,en;q=0.5")
        .header(header::ACCEPT_ENCODING, "gzip")
      .header(header::USER_AGENT,"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/101.0")
      .query(query)
      .send()?
      .text()?)

}

pub fn get_ep_html(ep_id: &str)-> Result<String,Box<dyn Error>>{
    let url= EP_URL.join(ep_id)?;
    simple_http_get(&url,&HashMap::new())
}

pub fn get_subtitle_list(bvid:&str,cid:u64)-> Result<Vec<SubtitleInfo>,Box<dyn Error>>{
    let mut query= HashMap::new();
    query.insert("bvid",bvid.to_string());
    query.insert("cid",cid.to_string());
    let content= simple_http_get(&PLAYER_URL,&query)?;
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
    
}
