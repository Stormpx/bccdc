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


