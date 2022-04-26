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
}

struct Context<'a>{
    dir: Option<&'a str>,
    subtitles: Vec<cc::CcSubtitle>,
}

fn parse_args(args: &mut std::env::Args)-> Result<(Config,Vec<String>),Box<dyn Error>> {

    let mut work_dir = std::env::current_dir().expect("fail to get pwd.");
    let mut format= String::from("srt");
    let mut proxy: Option<String> = None;
    args.next();
    let mut arg = args.next();
    let mut param: Vec<String> = Vec::new();
    while let Some(value) = arg{
        match value.as_str() {
            "-d" => {
                work_dir= Path::new(&args.next().ok_or("-d requires parameter")?).to_path_buf();
            },
            "--proxy" =>{
                proxy = Some(args.next().ok_or("--proxy requires parameter")?)
            },
            "-c" =>{
               format = args.next().ok_or("-c requires parameter")?;
            },
            other => {
                param.push(other.to_string());
                args.into_iter().for_each(|x| param.push(x));
            }
        }
        
        arg=args.next();
    }
    
    bili::init_client(proxy)?;

    Ok((Config{work_dir,format},param))
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

fn lookup_param<'a>(_config: &Config, param: &'a mut Vec<String>)->Result<Context<'a>,Box<dyn Error>>{
    let arg0= &param[0].trim();
    
    { 
        let target= arg0.to_lowercase();
        if target.starts_with("av") || target.starts_with("bv"){

            let mut ranges = param.iter()
                .skip(1)
                .filter(|x|!x.is_empty())
                .map(|x| parse_range(x))
                .collect::<Result<Vec<lookup::Page>,Box<dyn Error>>>()?;
            if ranges.is_empty(){
                ranges= vec![lookup::Page::All];
            }
            let subtitles : Vec<cc::CcSubtitle> = lookup::lookup_video_id(arg0,ranges)?
                .into_iter()
                .flat_map(|vp| {
                    let mut subs = vp.subtitles;
                    for sub in subs.iter_mut(){
                        sub.name = format!("{}-{}",vp.p,sub.name);
                    }
                    subs
                })
                .collect();
            return Ok(Context {
                    dir: Some(arg0),
                    subtitles: subtitles
                });
 
        }else if target.starts_with("ep"){
            let subtitles = lookup::lookup_ep_id(&target)?;
            return Ok(Context {
                    dir: Some(arg0),
                    subtitles:subtitles 
                });
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
                    Err(e) => eprintln!("fail to lookup {}. cause: {}",url,e),
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
                    Err(e) => eprintln!("fail to lookup file {}. cause: {}",path.display(),e),
                }
            });

    }
    Ok(Context { 
            dir: None , 
            subtitles: result,
       })


}

fn new_formatter(config: &Config)-> Box<dyn Formatter>{
    let format = config.format.to_lowercase() ;
    match format.as_str() {
        "srt"=> Box::new(cc::Srt::new()), 
        "sub"=> Box::new(cc::Sub::new()),
        "ass"=> Box::new(cc::Ass::new()),
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
                let context = match lookup_param(&config,&mut param){
                    Ok(v)=>v,
                    Err(e)=> {
                        eprintln!("{}",e);
                        process::exit(1);
                    }
                };
                write_context(&mut config,formatter.as_mut(),context);
            }else if let None = r {
                eprintln!("fail to parse input.");
            }

        }
    }else{
        let context = match lookup_param(&config,&mut param){
            Ok(v)=>v,
            Err(e)=> {
                eprintln!("{}",e);
                process::exit(1);
            }
        };
        write_context(&mut config,formatter.as_mut(),context);
    }
    
    
}

fn write_context(config: &mut Config, formatter: &mut dyn Formatter, context: Context){
    let work_dir= &mut config.work_dir;

    let subtitles = context.subtitles;
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

fn write_subtitle_to_file(file_path: &Path,subtitle: cc::CcSubtitle, formatter: &mut dyn cc::Formatter)-> std::io::Result<()>{
  let mut file = fs::File::create(file_path)?;
  formatter.write(&mut file,subtitle)?;
  Ok(())
}



