use std::collections::HashMap;
use std::{process,io,fs};
use url::{Url};
use std::path::{Path,PathBuf};
use std::error::Error;
use bccdc::cc;
use bccdc::cc::Formatter;
use bccdc::lookup;

use bccdc::bili;

struct Config{
    work_dir: PathBuf,
    format: String,
    doc: bool,
    mixed: bool,
}

impl Config{
    
    fn determine_name(&self , sub: &mut cc::CcSubtitle){
        let o_lan = if !self.doc { &sub.lan}else{ &sub.lan_doc};
        if let Some(lan) = o_lan {
            let name = &mut sub.name;
            name.clear();
            name.push_str(lan);
        }
    }
}

struct Context<'a>{
    dir: Option<&'a str>,
    subtitles: Vec<cc::CcSubtitle>,
}

fn print_helps(){

    println!("Usage: bccdc [option..] <avid/bvid/mdid/epid/bcc_url/bcc_file>

Examples:
    bccdc -d downloads/ --header 'cookie:value' BV1mT42127CQ
    bccdc -d downloads/ BV1ns411D7NJ 1 3-4 # download BV1ns411D7NJ p1 p3 p4
    bccdc -d downloads/ md28237168 2-3 9 # download md28237168 ep2 ep3 ep9
    bccdc -d downloads/ ep475901
    bccdc -d downloads/ subtitle.json
    bccdc --mixed -d dwonloads/ ep475901 BV1ns411D7NJ 3-4 md28237168 9 subtitle.json

Options:
    -d <directory> specify the output directory
    -c <srt/ass/vtt> specify the subtitle format to convert. default: srt 
    --doc use language_name as filename instead of language_tag. (take effect while downloading with bvid/epid)
    --mixed allow pass mixed arguments
    --proxy <http://host:port> use proxy
    --header <key:value> pass custom header to server"
);

    process::exit(0);
}

fn parse_args(args: &mut std::env::Args)-> Result<(Config,Vec<String>),Box<dyn Error>> {
    let mut work_dir = std::env::current_dir().expect("fail to get pwd.");
    let mut format= String::from("srt");
    let mut doc= false;
    let mut mixed = false;
    let mut proxy: Option<String> = None;
    let mut headers: HashMap<String,Vec<String>> = HashMap::new();
    args.next();
    let mut arg = args.next();
    let mut param: Vec<String> = Vec::new();
    while let Some(value) = arg{
        match value.as_str() {
            "-h"|"--help" => print_helps(),
            "-d" => {
                let p = args.next().ok_or("-d requires parameter")?;
                work_dir= Path::new(&p).to_path_buf();
            },
            "--proxy" =>{
                proxy = Some(args.next().ok_or("--proxy requires parameter")?);
            },
            "-H"|"--header"=>{
                let kv = args.next().ok_or("--header requires parameter")?;
                let split = kv.split_once(":").ok_or("--header requires pattern key:value")?;
                match headers.get_mut(split.0) {
                    Some(vals)=> vals.push(split.1.to_string()),
                    None=>{
                        headers.insert(split.0.to_string(), vec![split.1.to_string()]);
                    }
                }
            },
            "-c" =>{
               format = args.next().ok_or("-c requires parameter")?;
            },
            "--mixed" =>{
                mixed= true;
            },
            "--doc" =>{
                doc = true;
            },
            _ => {
                param.push(value);
                args.into_iter().for_each(|x| param.push(x));
            }
        }
        
        arg=args.next();
    }
    
    bili::init_client(proxy,headers)?;

    Ok((Config{work_dir,format,doc,mixed},param))
}

fn parse_range(string: &str)-> Result<lookup::Page,Box<dyn Error>>{
    let string= string.trim();
    if let Ok(p) = string.parse::<i32>(){
        if p<0 {
            return Err("expected positive int".into());
        }
        return Ok(lookup::Page::Single(p as u32));
    }
    if let Some((s,e)) = string.split_once("-"){
        if let (Ok(mut s),Ok(mut e)) = (s.parse::<u32>(), e.parse::<u32>()) {
            if s>e{
                (s,e)=(e,s);
            }
            return Ok(lookup::Page::Range(s,e));
        }
    }
    return Err(format!("expected <p> or <p-p>. but found {}",string).into());
    
}

fn lookup_mixed_param<'a>(config: &Config, param: &'a mut Vec<String>)->Result<Vec<Context<'a>>,Box<dyn Error>>{

    let mut result = vec![];
    let mut params = param.iter();
    let mut val_opt = params.next();
    while let Some(val) = val_opt {
        { 
            let target= val.to_lowercase();
            if target.starts_with("av") || target.starts_with("bv") || target.starts_with("md"){
                
                let mut ranges = vec![];
                let mut next_value = None;
                //parse range  
                while let Some(next) = params.next(){
                    if let Ok(p)= parse_range(next){
                       ranges.push(p);
                    }else{
                        next_value = Some(next);
                        break;
                    }
                }

                if ranges.is_empty(){
                    ranges.push(lookup::Page::All);
                }

                let vps = if target.starts_with("md"){
                    lookup::lookup_media_id(val,ranges)?
                }else{
                    lookup::lookup_video_id(val,ranges)?
                };

                let subtitles : Vec<cc::CcSubtitle> = vps.into_iter()
                    .flat_map(|vp| {
                        let mut subs = vp.subtitles;
                        for sub in subs.iter_mut(){
                            config.determine_name(sub);    
                            sub.name = format!("{}-{}",vp.p,sub.name);
                        }
                        subs
                    })
                    .collect();

                 result.push(Context {
                        dir: Some(val),
                        subtitles: subtitles
                 });
                if next_value.is_some(){
                    val_opt=next_value;
                }else{
                    val_opt=params.next();
                }

                continue;
 
            }
            if target.starts_with("md"){

            }
            if target.starts_with("ep"){
                let mut subtitles = lookup::lookup_ep_id(&target)?;
                for sub in subtitles.iter_mut(){
                    config.determine_name(sub);    
                }

                result.push(Context {
                    dir: Some(val),
                    subtitles:subtitles 
                });
                val_opt = params.next();
                continue; 
            }
        }

        let mut subtitle = None; 
        if let Ok(url) = Url::parse(val){
            match lookup::lookup_cc_api(&url){
                Ok(sub) => subtitle = Some(sub),
                Err(e) => eprintln!("fail to lookup {}: {}",url,e),
            }

        }else{
        //fallback to 'path'
            let path = Path::new(val);
            match lookup::lookup_file(&path){
                Ok(sub) => subtitle = Some(sub),
                Err(e) => eprintln!("{}: {}",path.display(),e),
            }

        }
        if let Some(subtitle) = subtitle{
            result.push(Context { 
                dir: None , 
                subtitles: vec![subtitle],
            })
        }

        val_opt = params.next();


    }
    Ok( result )

}



fn lookup_param<'a>(config: &Config, param: &'a mut Vec<String>)->Result<Vec<Context<'a>>,Box<dyn Error>>{
    let arg0= &param[0].trim();
    
    { 
        let target= arg0.to_lowercase();
        if target.starts_with("av") || target.starts_with("bv") || target.starts_with("md"){

            let mut ranges = param.iter()
                .skip(1)
                .filter(|x|!x.is_empty())
                .map(|x| parse_range(x))
                .collect::<Result<Vec<lookup::Page>,Box<dyn Error>>>()?;
            if ranges.is_empty(){
                ranges= vec![lookup::Page::All];
            }
            let vps = if target.starts_with("md"){ 
                lookup::lookup_media_id(arg0,ranges)?
            } else {
                lookup::lookup_video_id(arg0,ranges)?
            };

            let subtitles : Vec<cc::CcSubtitle> = vps.into_iter()
                .flat_map(|vp| {
                    let mut subs = vp.subtitles;
                    for sub in subs.iter_mut(){
                        config.determine_name(sub);    
                        sub.name = format!("{}-{}",vp.p,sub.name);
                    }
                    subs
                })
                .collect();
            return Ok(vec![
                Context {
                    dir: Some(arg0),
                    subtitles: subtitles
                }
                ]);
 
        }else if target.starts_with("ep"){
            let mut result = vec![];
            param.iter()
                .for_each(|target| match lookup::lookup_ep_id(&target.to_lowercase()){
                    Ok(mut subtitles)=>{
                        for sub in subtitles.iter_mut(){
                            config.determine_name(sub);    
                        }

                        result.push(Context {
                            dir: Some(target),
                            subtitles: subtitles
                        });
    
                    },
                    Err(e) => eprintln!("fail to lookup {}: {}",target,e),
                });

            return Ok(result);
            
        }
    }

    let mut result= Vec::new();
    if let Ok(_url) = Url::parse(arg0){
        param.iter()
            .map(|x|x.trim())
            .filter(|x|!x.is_empty())
            .map(|x| {
                match Url::parse(x){
                    Ok(url) =>Some( url ), 
                    Err(_)=>{
                        eprintln!("fail to parse {}",x);
                        None
                    },
                }
            })
            .filter(|r|r.is_some())
            .map(|r|r.unwrap())
            .for_each(|url|{
                match lookup::lookup_cc_api(&url){
                    Ok(subtitle) => result.push(subtitle),
                    Err(e) => eprintln!("fail to lookup {}: {}",url,e),
                }
            });
        
    }else{
        //fallback to 'path'
        param.iter()
            .map(|x|x.trim())
            .filter(|x|!x.is_empty())
            .map(|x|Path::new(x))
            .for_each(|path| {
                match lookup::lookup_file(&path){
                    Ok(subtitle) => result.push(subtitle),
                    Err(e) => eprintln!("{}: {}",path.display(),e),
                }
            });

    }
    Ok(vec![Context { 
            dir: None , 
            subtitles: result,
       }])


}

fn new_formatter(config: &Config)-> Box<dyn Formatter>{
    let format = config.format.to_lowercase() ;
    match format.as_str() {
        "srt"=> Box::new(cc::Srt::new()), 
        "sub"=> Box::new(cc::Sub::new()),
        "ass"=> Box::new(cc::Ass::new()),
        "vtt"=> Box::new(cc::Vtt::new()),
        other => {
            eprintln!("unsupported subtitle format {}",other);
            process::exit(1);
        }

    }

}

fn main() {
    
    let mut args = std::env::args();
    let (mut config,mut param) = match parse_args(&mut args){
        Ok((config,param))=> (config,param),
        Err(e) => {
            eprintln!("{}",e);
            process::exit(1);
        }
    };


    let mut formatter = new_formatter(&config);

    if param.is_empty(){
        loop{
            let mut input = String::new();
            let n = match io::stdin().read_line(&mut input){
                Ok(n)=>n,
                Err(e)=>{
                    eprintln!("{}",e);
                    process::exit(1);
                }
            };
            if n==0{
                break;
            }
            let r = shlex::split(&input);
            if let Some(mut param) = r{
                if param.is_empty(){
                    continue;
                }

                let result = if config.mixed{ lookup_mixed_param(&config,&mut param)}else{ lookup_param(&config,&mut param) };
                let contexts = match result {
                    Ok(v)=>v,
                    Err(e)=> {
                        eprintln!("{}",e);
                        process::exit(1);
                    }
                };
                contexts.iter().for_each(|context| write_context(&mut config,formatter.as_mut(),context));
            }else if let None = r {
                eprintln!("fail to parse input.");
            }

        }
    }else{
        let result = if config.mixed{ lookup_mixed_param(&config,&mut param)}else{ lookup_param(&config,&mut param) };
        let contexts = match result {
            Ok(v)=>v,
            Err(e)=> {
                eprintln!("{}",e);
                process::exit(1);
            }
        };

        contexts.iter().for_each(|context| write_context(&mut config,formatter.as_mut(),context));
    }
    
    
}

fn write_context(config: &mut Config, formatter: &mut dyn Formatter, context:&Context){
    let work_dir= &mut config.work_dir;

    let subtitles = &context.subtitles;
    if subtitles.is_empty(){
        return; 
    }
    if let Some(dir) = context.dir{
        work_dir.push(dir);
    }

     let dir = work_dir.as_path();
     if !dir.exists() {
        if let Err(e) = fs::create_dir_all(dir){
            eprintln!("{}",e);
            process::exit(1);

        }
     }

    for subtitle in subtitles{

        work_dir.push(subtitle.name.clone());
        work_dir.set_extension(formatter.ext());

        let path = work_dir.as_path();
        write_subtitle_to_file(&path,subtitle,formatter)
          .expect("fail to write subtitle file");

        println!("{}",path.display());
        work_dir.pop();

    }
    if let Some(_) = context.dir{
        work_dir.pop();
    }
     
}

fn write_subtitle_to_file(file_path: &Path,subtitle: &cc::CcSubtitle, formatter: &mut dyn cc::Formatter)-> std::io::Result<()>{
  let mut file = fs::File::create(file_path)?;
  formatter.write(&mut file,subtitle)?;
  Ok(())
}



