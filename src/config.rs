use std::str;

use nom::{IResult, multispace, space};
use nom::IResult::*;
use nom::{InputLength, IterIndices};
use nom::{AsChar, ErrorKind};
use nom::Err::Position;

use std::io::prelude::*;
use std::collections::HashMap;
use std::ops::{Index, Range, RangeFrom};

fn end_of_symbol(chr: char) -> bool {
  !(chr == ' ' || 
   chr == '#' || 
   chr == '\n' || 
   chr == '=' ||  // this is to work in key names. Will have to fixe someday
   chr == '{' ||
   chr == '}' )
}

pub fn until_eol<'a, T: ?Sized>(input: &'a T) -> IResult<&'a T, &'a T>
    where T: Index<Range<usize>, Output = T> + Index<RangeFrom<usize>, Output = T>,
          &'a T: IterIndices + InputLength
{
    let input_length = input.input_len();
    if input_length == 0 {
        return Error(Position(ErrorKind::MultiSpace, input));
    }

    let mut in_comment = false;

    for (idx, item) in input.iter_indices() {
        let chr = item.as_char();
        if chr == '#' {
            in_comment = true;
        }
        if in_comment {
            if chr == '\n' {
                return Done(&input[idx..], &input[0..idx]);
            }
        } else {
            if !(chr == ' ' || chr == '\t') || chr == '\n' {
                    return Done(&input[idx..], &input[0..idx]);
            }
        }
    }
    Done(&input[input_length..], input)
}

fn keys_and_values(input: &str) -> IResult<&str, HashMap<String, String>> {
    let mut h: HashMap<String, String> = HashMap::new();

    match keys_and_values_aggregator(input) {
        IResult::Done(i, tuple_vec) => {
            for &(k, v) in tuple_vec.iter() {
                h.insert(k.to_owned().into(), v.to_owned().into());
            }
            IResult::Done(i, h)
        }
        IResult::Incomplete(a) => IResult::Incomplete(a),
        IResult::Error(a) => IResult::Error(a),
    }
}

named!(pub quoted_string <&str, &str>,
       chain!(
         tag_s!("\"")              ~
         qs: take_until_s!("\"")   ~
         tag_s!("\"")              ,
         || { qs }
         )
      );

// A symbol is anything between spaces, and followed by something.
named!(object_symbol_name <&str, &str>,
       chain!(
         multispace? ~
         symbol: take_while_s!( end_of_symbol ),
           || { (symbol) } ));


named!(pub multispace_and_comment <&str, Vec<&str> >, many1!(until_eol));

named!(pub declaration <&str, &str>,
       chain!(
         symbol: object_symbol_name   ~
         kv: chain!(
             multispace_and_comment       ~
             tag_s!("{")               ~
             kv: keys_and_values       ~
             tag_s!("}"),
             || { kv }
             )?
         ,
         || {
           println!("Parms: {:?}", kv);
           symbol })
      );

named!(pub key_value    <&str,(&str,&str)>,
chain!(
  key: object_symbol_name             ~
  space?                              ~
  tag_s!("=")                         ~
  space?                              ~
  val: alt!(
    quoted_string                     |
    object_symbol_name
    )                                 ~
  multispace_and_comment              ,
  ||{(key, val)}
  )
);

named!(pub keys_and_values_aggregator<&str, Vec<(&str,&str)> >,
chain!(
  kva: many0!(key_value),
  || {kva} )
);

use std::fs::File;

#[derive(Debug)]
pub struct Configuration {
  pub inputs: Vec<(String,  Option<HashMap<String,String>>)>,
  pub outputs: Vec<(String,  Option<HashMap<String,String>>)>,
}

pub fn read_config_file(filename: &str) -> Result<Configuration, String> {
  println!("Reading config file.");
  let mut f = File::open(filename).unwrap();
  let mut s = String::new();

  match f.read_to_string(&mut s) {
    Ok(_) => Ok(Configuration{ inputs: vec![], outputs: vec![] }),
    Err(e) => Err(format!("Read error: {:?}", e))
  }
}
