use std::process;
use std::fs;
use url::{Url};
use std::io::prelude::*;
use std::path::{Path,PathBuf};
use std::error::Error;

use bccdc::cc;
use bccdc::lookup;

struct Config{
    work_dir: PathBuf,
}


fn parse_args(args: &mut std::env::Args)-> Result<(Config,Vec<String>),Box<dyn Error>> {

    let mut work_dir = std::env::current_dir().expect("fail to get pwd.");
    args.next();
    let mut arg = args.next();
    let mut param: Vec<String> = Vec::new();
    while let Some(value) = arg{
        if value == "-d"{
            work_dir= Path::new(&args.next().ok_or("-d required param")?).to_path_buf();
        }else{
            param.push(value);
            args.into_iter().for_each(|x| param.push(x));
        }
        arg=args.next();
    }


    Ok((Config{work_dir},param))
}

fn lookup_param(config: &Config, param: &mut Vec<String>)->Result<Vec<cc::CcSubtitle>,Box<dyn Error>>{
    let arg0= &param[0];
    
    { 
        let target= arg0.to_lowercase();
        if target.starts_with("av") || target.starts_with("bv"){
            return Err("not supported yet.".into());
        }else if target.starts_with("ep"){
            return Err("not supported yet.".into());
        }
    }

    let mut result= Vec::new();
    if let Ok(_url) = Url::parse(arg0){
        param.iter()
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
            .map(|x|Path::new(x))
            .for_each(|path| {
                match lookup::lookup_file(&path){
                    Ok(subtitle) => result.push(subtitle),
                    Err(e) => eprintln!("fail to lookup {}. cause: {}",path.display(),e),
                }
            });

    }
    Ok(result)

}

fn main() {
    
    let mut args = std::env::args();
    let (config,mut param) = parse_args(&mut args).expect("");

    if param.is_empty(){
        return ;
    }
    //let input = &param[0];

    //let url = Url::parse(input).expect("invalid url.");
    //let subtitles = lookup::lookup_cc_api(vec![&url]).expect("");
    let  subtitles= match lookup_param(&config,&mut param){
        Ok(v)=>v,
        Err(e)=> {
            println!("{}",e);
            process::exit(1);
        }
    };

    if subtitles.is_empty(){
        return; 
    }

    let mut work_dir= config.work_dir;
    
    for subtitle in subtitles{
        work_dir.push(subtitle.name.clone());
        write_subtitle_to_file(work_dir.as_path(),subtitle)
          .expect("fail to write subtitle file");

        println!("{}",work_dir.as_path().display());
        work_dir.pop();

    }
    
}

fn write_subtitle_to_file(file_path: &Path,subtitle: cc::CcSubtitle)-> std::io::Result<()>{
  let mut file = fs::File::create(file_path.with_extension("srt").as_path())?;
  for (index,line) in subtitle.lines.iter().enumerate(){

    let str=format!("{}\n\
        {} --> {}\n\
        {}\n\n",index+1,line.format_start(),line.format_end(),line.content
        );
    file.write(str.as_bytes())?;
  }

  Ok(())
}



