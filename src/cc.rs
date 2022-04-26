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

pub trait Formatter{

    fn ext(&self)->&str;

    fn write(&mut self, writer: &mut dyn  Write, subtitle: CcSubtitle)-> Result<(),Error>;
    
}
pub struct Srt {
}
impl Srt{
    
    pub fn new()->Self{
        Srt{}
    }

    fn format_time(time: f64)-> String{
        let hour = time as u64 /3600;
        let minute = time as u64 /60 %60;
        let second = time as u64 %60;
        let ms = time.fract().mul_add(1000.,0.)as u64;
        format!("{:0>2}:{:0>2}:{:0>2},{:0>3}",hour,minute,second ,ms)

    }
}

impl Formatter for Srt{


    fn ext(&self)->&str{
        "srt"
    }

    fn write(&mut self, writer: &mut dyn  Write, subtitle: CcSubtitle)-> Result<(),Error>{
        for (index, line ) in subtitle.lines.iter().enumerate(){
            let str=format!("{}\n\
                {} --> {}\n\
                {}\n\n",index+1,Srt::format_time(line.start),Srt::format_time(line.end),line.content
                );

            writer.write(str.as_bytes())?;
        }
        Ok(())
    }
    


}

pub struct Sub{

}
impl Sub{
    
    pub fn new()->Self{
        Sub{}
    }


    fn to_frame(second: f64)-> u64{
        (second * 23.976) as u64

    }
}



impl Formatter for Sub{


    fn ext(&self)->&str{
        "sub"
    }

    fn write(&mut self, writer: &mut dyn  Write, subtitle: CcSubtitle)-> Result<(),Error>{
        for  line  in subtitle.lines.iter(){
            let str=format!(
                "{{{}}}{{{}}}{}\n\n",
                Sub::to_frame(line.start),Sub::to_frame(line.end),line.content
                );

            writer.write(str.as_bytes())?;
        }
        Ok(())
    }

}

pub struct Ass{
}
impl Ass{
    pub fn new()->Self{
        Ass{}
    }

    fn format_time(time: f64)-> String{
        let hour = time as u64 /3600;
        let minute = time as u64 /60 %60;
        let second = time as u64 %60;
        let hunderdths = (time.fract() * 100.) as u64; 
        format!("{}:{:0>2}:{:0>2}.{:0>2}",hour,minute,second,hunderdths)
    }
    
    fn format_line(line: &Line) -> String{
       format!("Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            Ass::format_time(line.start),Ass::format_time(line.end),line.content.replace("\n","\\n"))
        
    }
    fn write_header(&mut self,writer: &mut dyn Write,subtitle:&CcSubtitle) -> Result<(),Error>{
        
        writer.write(b"[Script Info]\n")?;
        writer.write(b"; Generated by bccdc.\n")?;
        writer.write(format!("Title: {}\n",subtitle.name).as_bytes())?;
        writer.write(b"ScriptType: v4.00+\n")?;
        writer.write(b"WrapStyle: 2\n")?;

        writer.write(b"\n")?;

        writer.write(b"[V4+ Styles]\n")?;
        writer.write(b"Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n")?;
        
        writer.write(b"Style: Default,Arial,20,&H00FFFFFF,&H0000FFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,1,2,10,10,10,1\n")?;
        writer.write(b"\n")?;
        Ok(())
    }

}

impl Formatter for Ass{
    fn ext(&self)->&str{
        "ass"
    }

    fn write(&mut self, writer: &mut dyn  Write, subtitle: CcSubtitle)-> Result<(),Error>{
        self.write_header(writer,&subtitle)?;
        writer.write(b"[Events]\n")?;
        writer.write(b"Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n")?;

        for line in subtitle.lines.iter(){
            let str=Ass::format_line(&line);
            writer.write(str.as_bytes())?;
        }
        Ok(())
    }

}
