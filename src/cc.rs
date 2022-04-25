use std::io::Error;
use std::io::Write;

pub struct CcSubtitle{
    pub name: String,
    pub lines: Vec<Line>,
}

#[derive(Debug)]
pub struct Line{
    pub content: String,
    pub start: f64,
    pub end: f64,
}
impl Line{
    pub fn format_start(&self)->String {
        let second = self.start as u64 %60;
        let minute = self.start as u64 /60 %60;
        let hour = self.start as u64 /3600;
        let decimal = self.start.fract().mul_add(1000.,0.)as u64;
        format!("{:0>2}:{:0>2}:{:0>2},{:0>3}",hour,minute,second ,decimal)
    }
    pub fn format_end(&self)->String {
        let second = self.end as u64 %60;
        let minute = self.end as u64 /60 %60;
        let hour = self.end as u64 /3600;
        let decimal = self.end.fract().mul_add(1000.,0.)as u64;
        format!("{:0>2}:{:0>2}:{:0>2},{:0>3}",hour,minute,second ,decimal)
    }

}


pub trait Formatter{
    

    fn ext(&self)->&str;

    fn format(&mut self , subtitle: &CcSubtitle)->Vec<String>;

    fn format_line(&mut self, line: &Line)-> String;

    fn write(&mut self, writer: &mut dyn  Write, subtitle: CcSubtitle)-> Result<(),Error>{
        for line in subtitle.lines{
            writer.write(self.format_line(&line).as_bytes())?;
        }
        Ok(())
    }
    
}
pub struct Srt {
    counter: u32,
}
impl Srt{
    
    pub fn new()->Srt{
        Srt{counter: 1}
    }
}

impl Formatter for Srt{


    fn ext(&self)->&str{
        "srt"
    }

    fn format_line(&mut self, line: &Line)-> String{
        let str=format!("{}\n\
                {} --> {}\n\
                {}\n\n",self.counter,line.format_start(),line.format_end(),line.content
                );

        self.counter+=1;
        str
    }

    fn format(&mut self , subtitle: &CcSubtitle)->Vec<String>{
        let mut result = Vec::new();
        for line in subtitle.lines.iter(){
            result.push(self.format_line(line));
        }
        result
    }

}

pub struct Vtt{
}

