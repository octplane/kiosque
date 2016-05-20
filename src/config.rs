use std::str;

use nom::{IResult, multispace, alphanumeric, space};
use nom::IResult::*;

use std::io::prelude::*;
use std::collections::HashMap;

macro_rules! test_gen { ($t:expr, $fun:expr, [ $( $it:expr ),* ])   => {
  $(
    {
      let res = $fun($it);
      if let Done(_,p) = res {
        println!("{}: {:?}", $t, p);
      } else {
        assert!(false, format!("{}: Failed to parse correctly {:?}: {:?}", $t, $it, res));
      }
    }
   )*
}
}


named!(quoted_string <&str, &str>,
       chain!(
         tag_s!("\"")              ~
         qs: take_until_s!("\"")   ~
         tag_s!("\"")              ,
         || { qs }
         )
      );

#[test]
fn test_quoted_string() {
    test_gen!(
    "quoted_string",
    quoted_string,
    [
    "\"value of parameter\"",
    "\"🍓\""
    ]
    );
}

fn end_of_symbol(chr: char) -> bool {
  !(chr == ' ' || 
   chr == '#' || 
   chr == '\n' || 
   chr == '{' ||
   chr == '}' )
}


// A symbol is anything between spaces, and followed by something.
named!(object_symbol_name <&str, &str>,
       chain!(
         multispace? ~
         symbol: take_while_s!( end_of_symbol ),
           || { (symbol) } ));


#[test]
fn test_object_symbol_name() {

    test_gen!(
    "object_symbol_name",
    object_symbol_name,
    [
    "👔\n",
    " this_is_valid_symbol ",
    "this_is_a_valid_symbol {",
    "this_is_a_valid_symbol}",
    "symbol\n\tshould_not_show=12\noutput_folder = \"./logs/$APP/\"\n"
    ]
    );

}

use std::ops::{Index, Range, RangeFrom};
use nom::{InputLength, IterIndices};
use nom::{AsChar, ErrorKind};
use nom::Err::Position;

/// Recognizes spaces, tabs, carriage returns and line feeds
/// Detect # in multispace content and then eats until \n
pub fn multispace_and_comment<'a, T: ?Sized>(input: &'a T) -> IResult<&'a T, &'a T>
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
                in_comment = false
            }
        } else {
            if !(chr == ' ' || chr == '\t' || chr == '\n') {
                if idx == 0 {
                    return Error(Position(ErrorKind::MultiSpace, input));
                } else {
                    return Done(&input[idx..], &input[0..idx]);
                }
            }
        }
    }
    Done(&input[input_length..], input)
}

#[test]
fn test_multispace_content() {
    test_gen!(
    "multispace_and_comment",
    multispace_and_comment,
    [
    "   ",
    "  \n  ",
    "\t \n ",
    "#",
    "   # this is a sample comment",
    "   # this is a sample comment\n# with multiline things\n  \t",
    "\n#\n# \n\t#\n"]);
}

named!(declaration <&str, &str>,
       chain!(
         symbol: object_symbol_name   ~
         alt!(
           chain!(
             tag_s!("{")               ~
             kv: keys_and_values       ~
             tag_s!("}"),
             || { kv }
             )                         |
           multispace_and_comment => { |_| HashMap::<String,String>::new()}
           )
         ,
         || { symbol })
      );

#[test]
fn test_declarations() {
    test_gen!(
    "declaration",
    declaration,
    [
    "empty_declaration\n",
    "empty_declaration # with a comment\n",
    "💩 {}",
    "         💩 {}",
    " 💩 { \n }",
    " 💩  # coucou\n{ \n }",
    " 💩 { # 🍓 \n }",
    " 💩 { \n # coucou \n  }"]);
}


named!(key_value    <&str,(&str,&str)>,
chain!(
  key: alphanumeric                   ~
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

#[test]
fn test_key_value() {
  test_gen!(
    "key_value",
    key_value,
    [
      "foo = \"bar\"  \n  ",
      "length=12\n",
      "length = 12\n",
      "length = 12\n",
      "length = douze\n",
      "length = 🍓\n",
      "length = \"🍓\"\n",
      "length = \"🍓\" # and now with some comment\n"
    ]);
}

named!(keys_and_values_aggregator<&str, Vec<(&str,&str)> >,
chain!(
  kva: many0!(key_value),
  || {kva} )
);

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

#[test]
fn test_keys_and_values_aggregator() {
  test_gen!(
    "keys_and_values_aggregator",
    keys_and_values_aggregator,
    [
      "foo = bar\n\tlength=12\noutput_folder = \"./logs/$APP/\"\n}"
    ]
  );
}


// named!(object_and_params <&str, (String, Option<HashMap<String,String>>)>,
//   chain!(
//     blanks                          ~
//     ik: object_symbol_name          ~
//     blanks                          ~
//     kv: keys_and_values?            ~
//     blanks                          ,
//     || { (ik.to_lowercase(), kv) }
//   )
// );
//
// named!(inputs <&str, Vec<(String, Option<HashMap<String,String>>)> >,
//   chain!(
//     tag!("input")                     ~
//     blanks                            ~
//     tag!("{")                         ~
//     blanks                            ~
//     ins: many0!(object_and_params)     ~
//     blanks                            ~
//     tag!("}")                         ~
//     blanks                            ,
//     || { ins }
//   )
// );
//
// named!(outputs <&str, Vec<(String, Option<HashMap<String,String>>)> >,
//   chain!(
//     tag!("output")                     ~
//     blanks                            ~
//     tag!("{")                         ~
//     blanks                            ~
//     outs: many0!(object_and_params)     ~
//     blanks                            ~
//     tag!("}")                         ~
//     blanks                            ,
//     || { outs }
//   )
// );
//
// #[derive(Debug)]
// pub struct Configuration {
//   pub inputs: Vec<(String,  Option<HashMap<String,String>>)>,
//   pub outputs: Vec<(String,  Option<HashMap<String,String>>)>,
// }
//
// named!(configuration  <&str, Configuration>,
//   chain!(
//     inputs: inputs          ~
//     blanks                  ~
//     outputs: outputs        ,
//     || {
//       Configuration{
//         inputs: inputs,
//         outputs: outputs
//       }
//     }
//   )
// );
//
//
//
// pub fn read_config_file(filename: &str) -> Result<Configuration, String> {
//   println!("Reading config file.");
//   let mut f = File::open(filename).unwrap();
//   let mut s = String::new();
//
//   match f.read_to_string(&mut s) {
//     Ok(_) => {
//       match configuration(&s) {
//         Done(_, configuration) => Ok(configuration),
//         Error(e) => {
//           Err(format!("Parse error: {:?}", e))
//         },
//         Incomplete(e) => {
//           Err(format!("Incomplete content -> await: {:?}", e))
//         }
//       }
//     },
//     Err(e) => Err(format!("Read error: {:?}", e))
//   }
// }
//
// #[test]
// fn test_simple_config_parser() {
//   if let Ok(c) = read_config_file("tests/simple.conf") {
//   } else {
//     assert!(false);
//   }
// }
//
// #[test]
// fn test_config_parser() {
//   match read_config_file("tests/test_config.conf") {
//     Ok(conf) => {
//       println!("{:?}", conf);
//       // Some({"path": "some literal string", "pipo": "12"})), (Stdin, Some({"tag": "stdin"}))]
//       assert_eq!(conf.inputs.len(), 2);
//       assert_eq!(conf.inputs[0].0, "file");
//       let mut file_conf = HashMap::new();
//       file_conf.insert("path".to_owned(), "some literal string".to_owned());
//       file_conf.insert("pipo".to_owned(), "12".to_owned());
//       assert_eq!(conf.inputs[0].1, Some(file_conf) );
//
//       assert_eq!(conf.inputs[1].0, "stdin");
//       let mut stdin_conf = HashMap::new();
//       stdin_conf.insert("tag".to_owned(), "stdin".to_owned());
//       assert_eq!(conf.inputs[1].1, Some(stdin_conf) );
//
//
//     },
//     Err(e) => assert!(false, format!("Unable to parse configuration file: {}", e))
//   }
// }
