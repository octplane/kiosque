extern crate nom;
extern crate log_archive;

use log_archive::config::*;

macro_rules! test_gen { ($t:expr, $fun:expr, [ $( $it:expr ),* ])   => {
  $(
    {
      let res = $fun($it);
      if let Done(r,p) = res {
        println!("
>>>>>>>>>>>>>>
Parsing a {}:
\"\"
{}
\"\"
Found. {:?}
Remaining: {:?}
<<<<<<<<<<<<<<
", $t, $it, p, r);
      } else {
        assert!(false, format!("{}: Failed to parse correctly {:?}: {:?}", $t, $it, res));
      }
    }
   )*
}
}

macro_rules! test_gen_complete { ($t:expr, $fun:expr, [ $( $it:expr ),* ])   => {
  $(
    {
      let res = $fun($it);
      if let Done(r,_) = res {
        assert!(0 == r.len(), format!("{}: Rest is not empty: \"{}\" for {}", $t, r, $it));
      } else {
        assert!(false, format!("{}: Failed to parse correctly {:?}: {:?}", $t, $it, res));
      }
    }
   )*
}
}

#[cfg(test)]
mod config_test {
  use super::*;
  use std::collections::HashMap;

  use nom::IResult::*;

  use log_archive::config::*;


  #[test]
  fn test_multispace_content() {
      test_gen!(
      "multispace_and_comment",
      multispace_and_comment,
      [
      "\n",
      "\n\n",
      "   ",
      "  \n  ",
      "\t \n ",
      "#",
      "   # this is a sample comment",
      "   # this is a sample comment\n# with multiline things\n  \t",
      "\n#\n# \n\t#\n"]);
  }

  #[test]
  fn test_declarations() {
      test_gen!(
      "declaration",
      declaration,
      [
      "splunk { }",
      "empty_declaration   \n",
      "empty_declaration # with a comment\n",
      "💩 {}",
      "         💩 {}",
      " 💩 { \n }",
      " 💩  # coucou\n{ \n }",
      " 💩 { # 🍓 \n }",
      " splunk { \n foo=bar\n # 🍓 \n }",
      " 💩 { \n # coucou \n  }"]);
  }


  #[test]
  fn test_key_value() {
    test_gen!(
      "key_value",
      key_value,
      [
        "foo = \"bar\"  \n",
        "f_a = 12\n",
        "length=12\n",
        "length = 12\n",
        "length = 12\n",
        "length = douze\n",
        "length = 🍓\n",
        "length = \"🍓\"\n",
        "length = \"🍓\" # and now with some comment\n"
      ]);
  }

  #[test]
  fn test_keys_and_values_aggregator() {
    test_gen!(
      "keys_and_values_aggregator",
      keys_and_values_aggregator,
      [
        "foo = bar\n\tlength=12\noutput_folder = \"./logs/$APP/\"   \n👔 = OFF #Who cares ?\n}"
      ]
    );
  }
}

