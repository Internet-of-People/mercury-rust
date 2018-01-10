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
    // TODO should we return error instead?
    pub fn resolve_format<'d>(&self, format_id: &'d str, data: &'d [u8])
        -> Result< Box<Data + 'd>, AddressResolutionError >
    {
        let parser = self.formats.get(format_id)
            .ok_or( AddressResolutionError::UnknownFormat( format_id.to_owned() ) )?;
        let parsed_data = parser.parse(data)
            .map_err( |e| AddressResolutionError::FormatParserError(e) );
        parsed_data
    }
}



pub trait FormatParser
{
    fn parse<'b>(&self, blob: &'b [u8])
        -> Result< Box<Data + 'b>, FormatParserError >;
}