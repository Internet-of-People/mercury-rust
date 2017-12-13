use std::collections::HashMap;

use common::Data;
use error::*;



pub type FormatId = String;

pub struct FormatRegistry
{
    formats: HashMap< FormatId, Box<FormatParser> >
}

impl FormatRegistry
{
    pub fn formats(&self) -> &HashMap< FormatId, Box<FormatParser> >
        { &self.formats }
}



pub trait FormatParser
{
    fn parse(&self, blob: &[u8])
        -> Result< Box<Data + 'static>, FormatError >;
}