// The std::process::Command builder handles args in a way that is potentially
// more convenient than passing a full vector of args to the builder all at
// once.
//
// Look for a field attribute #[builder(each = "...")] on each field. The
// generated code may assume that fields with this attribute have the type Vec
// and should use the word given in the string literal as the name for the
// corresponding builder method which accepts one vector element at a time.
//
// In order for the compiler to know that these builder attributes are
// associated with your macro, they must be declared at the entry point of the
// derive macro. Otherwise the compiler will report them as unrecognized
// attributes and refuse to compile the caller's code.
//
//     #[proc_macro_derive(Builder, attributes(builder))]
//
// These are called inert attributes. The word "inert" indicates that these
// attributes do not correspond to a macro invocation on their own; they are
// simply looked at by other macro invocations.
//
// If the new one-at-a-time builder method is given the same name as the field,
// avoid generating an all-at-once builder method for that field because the
// names would conflict.
//
//
// Resources:
//
//   - Relevant syntax tree type:
//     https://docs.rs/syn/2.0/syn/struct.Attribute.html

use derive_builder::Builder;

#[derive(Builder, Debug)]
#[cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
pub struct Command {
    executable: String,
    #[builder(each = "arg")]
    args: Vec<String>,
    #[builder(each = "env")]
    env: Vec<String>,
    current_dir: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::builder()
        .executable("bash".to_owned())
        .arg("/c".to_owned())
        .arg("whoami".to_owned())
        .env("LANG=C".to_owned())
        .build()?;

    println!("{:#?}", command);
    Ok(())
}
