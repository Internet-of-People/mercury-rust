use std::collections::HashMap;

use common::Data;



pub type FormatId = String;

pub struct FormatRegistry
{
    formats: HashMap< FormatId, Box<FormatParser> >
}



pub trait FormatParser
{
    fn parse<'a>(&self, blob: &'a [u8]) -> Box<Data + 'a>;
}