use std::str;

use nom::{IResult, multispace, eof, alphanumeric, space, not_line_ending};
use nom::IResult::*;

use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;

named!(quoted_string <&str>,
       chain!(
         tag!("\"")              ~
         qs: map_res!(
           take_until!("\""),
           str::from_utf8)       ~
         tag!("\"")              ,
         || { qs }
         )
      );

named!(object_symbol_name <&str, &str>,
       chain!(
         foo: multispace? ~
         symbol: alt!(
           take_until_s!(" ")   |
           take_until_s!("\n")  |
           take_until_s!("{")   |
           take_until_s!("#")
           ),

           || { (symbol) } ));

pub struct Node {
  pub name: String,
  pub content: Vec<String>
}

/// Recognizes spaces, tabs, carriage returns and line feeds
/// Detect # in multispace content and then eats until \n

use std::ops::{Index, Range, RangeFrom};
use nom::{InputLength, IterIndices};
use nom::{AsChar, ErrorKind};
use nom::Err::Position;

pub fn multispace_and_comment<'a, T: ?Sized>(input:&'a T) -> IResult<&'a T, &'a T>
where T:Index<Range<usize>, Output=T>+Index<RangeFrom<usize>, Output=T>,
      &'a T: IterIndices+InputLength {

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
      if ! (chr == ' ' || chr == '\t' || chr == '\n') {
        if idx == 0 {
          return Error(Position(ErrorKind::MultiSpace, input))
        } else {
          return Done(&input[idx..], &input[0..idx])
        }
      }
    }
  }
  Done(&input[input_length..], input)
}

#[test]
fn test_multispace_and_comment() {
  let testable = vec![
    "   ",
    "  \n  ",
    "\t \n ",
    "#",
    "   # this is a sample comment",
    "   # this is a sample comment\n# with multiline things\n  \t",
    "\n#\n# \n\t#\n"
  ];
  for t in testable {
    let res = multispace_and_comment(t);
    if let Done(r,p) = res {
      assert_eq!(p, t);
    } else {
      assert!(false, format!("Failed to parse correctly \"{}\": {:?}", t, res));
    }
  }
}

named!(declaration <&str, &str>, 
       chain!(
         multispace_and_comment?     ~
         symbol : object_symbol_name ~
         multispace_and_comment?     ~
         tag_s!("{")?                 ~
         multispace_and_comment?     ~
         tag_s!("}")?
         ,
         || { symbol }));


#[test]
fn test_symbol() {
  let t = " this_is_a_valid_symbol ";
  let s = object_symbol_name(t);
  assert!(s == Done(" ", "this_is_a_valid_symbol"));

  let t1 = " this_is_a_valid_symbol {";
  let s1 = object_symbol_name(t1);
  assert!(s1 == Done(" {", "this_is_a_valid_symbol"));
}

#[test]
fn test_declaration() {
  let testable = vec![
    " 💩 {}",
    " 💩 { \n }",
    " 💩  # coucou\n{ \n }",
    " 💩 { # 🍓 \n }",
    " 💩 { \n # coucou \n  }",
  ];
  for t in testable {
    let res = declaration(t);
    if let Done(r,p) = res {
      print!("{} => R{:?} P:{}",t, r, p); 
      assert!(true);
    } else {
      assert!(false, format!("Failed to parse correctly \"{}\": {:?}", t, res));
    }
  }
}

// 
// named!(comment,
//     chain!(
//         tag!("#")           ~
//         not_line_ending?    ~
//         alt!(eol | eof)     ,
//         || { None }));
// 
// named!(opening, tag!("{"));
// 
// named!(key_value    <&str,(&str,&str)>,
//   chain!(
//     key: map_res!(alphanumeric, str::from_utf8) ~
//       space?                            ~
//       tag!("=")                         ~
//       space?                            ~
//     val: alt!(
//       quoted_string |
//       map_res!(
//         take_until_either!("\n\r#"),
//         str::from_utf8
//       )
//       )                                    ~
//       blanks                               ,
//     ||{(key, val)}
//   )
// );
// 
// 
// named!(keys_and_values_aggregator<&str, Vec<(&str,&str)> >,
//  chain!(
//      tag!("{")                ~
//      blanks                   ~
//      kva: many0!(key_value)   ~
//      blanks                   ~
//      tag!("}")                ,
//  || {kva} )
// );
// 
// fn keys_and_values(input:&str) -> IResult<&str, HashMap<String, String> > {
//   let mut h: HashMap<String, String> = HashMap::new();
// 
//   match keys_and_values_aggregator(input) {
//     IResult::Done(i, tuple_vec) => {
//       for &(k,v) in tuple_vec.iter() {
//         h.insert(k.to_owned(), v.to_owned());
//       }
//       IResult::Done(i, h)
//     },
//     IResult::Incomplete(a)     => IResult::Incomplete(a),
//     IResult::Error(a)          => IResult::Error(a)
//   }
// }
// 
// 
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
