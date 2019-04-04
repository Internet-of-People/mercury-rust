#![allow(unused, non_snake_case, non_upper_case_globals)]

use std::collections::HashMap;

use crate::common::*;
use crate::error::*;
use crate::meta::AttributeValue;


pub const Format_Separator: char = '@';
pub const Path_Separator:   char = '/';

pub type FormatId = String;

pub struct FormatRegistry
{
    formats: HashMap< FormatId, Box<FormatParser> >
}

impl FormatRegistry
{
    pub fn new(formats: HashMap< FormatId, Box<FormatParser> >) -> Self
        { Self{ formats: formats } }


    // TODO should we return error instead?
    fn resolve_format<'d>(&self, format_id: &'d str, data: &'d [u8])
        -> Result< Box<Data + 'd>, AddressResolutionError >
    {
        let parser = self.formats.get(format_id)
            .ok_or( AddressResolutionError::UnknownFormat( format_id.to_owned() ) )?;
        let parsed_data = parser.parse(data)
            .map_err( |e| AddressResolutionError::FormatParserError(e) );
        parsed_data
    }


    // Expected attribute specifier format: formatId@path/to/hashlink/attribute
    pub fn resolve_attr_link<'a,'d>(&self, data: &'d Vec<u8>, attr_spec: &'a str)
        -> Result<String, AddressResolutionError>
    {
        // Separate format and attribute path
        let format_sep_idx = attr_spec.find(Format_Separator).unwrap_or( attr_spec.len() );
        let (format_id, prefixed_attr_path_str) = attr_spec.split_at(format_sep_idx);
        let attr_path_str = &prefixed_attr_path_str[1..];
        let attr_path: Vec<&str> = attr_path_str.split(Path_Separator).collect();

        // Parse blob to fetch attributes
        let parsed_data = self.resolve_format( format_id, data.as_slice() )?;

        // Resolve attribute path
        let attr_res = parsed_data.first_attrval_by_path( attr_path.as_slice() )
            .ok_or( AddressResolutionError::AttributeNotFound( attr_path_str.to_owned() ) );

        match attr_res {
            Ok( AttributeValue::Link(v) ) => Ok( v.to_owned() ),
            Ok(_) => Err(AddressResolutionError::WrongAttributeType),
            Err(e) => Err(e),
        }
    }

}



pub trait FormatParser
{
    fn parse<'b>(&self, blob: &'b [u8])
        -> Result< Box<Data + 'b>, FormatParserError >;
}