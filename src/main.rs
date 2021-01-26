//! Generate C# classes from the given JSON
//! Author: Daniel Mar
//! TODO: handle name collisions?
//! TODO: map Value::Null to Nullable<string>?

/// General Strategy:
/// 1. Use the serde crate to parse JSON into an abstract syntax tree
///
/// 2. Generate the Root class.
/// - find property names by descending the AST node until a
///   concrete type is found.
/// - if a concrete type is itself an object/custom type, add that
///   object's node to a queue (for later processing).
///
/// 3. Generate the remaining classes by repeating (2) on the queued nodes.
/// -  a la breadth-first search

use serde_json::{Value, Map};
use std::fmt::{Write, Error};
use std::collections::VecDeque;
use std::{fmt, io, fs};
use clap::{Arg, App};
use std::io::Read;

// ----------------------------------------------------------------------------

/// Represents a CSharpType
/// Primitive  :  string | int | uint | float | bool
/// Custom     :  user-defined | primitive | []
enum CSharpType {
    Primitive(&'static str), // eg. "float"
    Custom(String),          // eg. "HttpResponse[]"
}

// make printable/writable
impl fmt::Display for CSharpType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CSharpType::Primitive(typename) => write!(f, "{}", typename),
            CSharpType::Custom(typename) => write!(f, "{}", typename),
        }
    }
}

// define conversion to string slice
impl CSharpType {
    fn as_str(&self) -> &str {
        match *self {
            CSharpType::Primitive(s) => s,
            CSharpType::Custom(ref s) => s
        }
    }
}

// ----------------------------------------------------------------------------

fn main() {
    // CLI config
    let matches = App::new("paste_json")
        .version("0.1")
        .author("Daniel M. <dmar@uw.edu>")
        .about("Generates C# classes to represent the given JSON.")
        .after_help(r#"EXAMPLES:
        (1)   paste_json weather.json

        (2)   cat weather.json | paste_json"#)
        .arg(Arg::with_name("file")
            .help("The file containing the JSON object")
            .required(false))
        .get_matches();
    
    let input = if let Some(filename) = matches.value_of_os("file") {
        // read from file
        fs::read_to_string(filename)
    } else {
        // read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).and_then(|_| Ok(buffer))
    };
    
    // JSON as a string.
    let json = input.expect("Failed to read JSON.");
    // Abstract syntax tree corresponding to the JSON
    let ast: Value = serde_json::from_str(&json).expect("Failed to parse JSON.");
    
    // generate classes
    let mut generator = ClassGenerator::new();
    let classes = generator.generate(&ast).expect("Failed to generate classes.");
    
    // print the classes
    print!("{}", classes);
}

// ----------------------------------------------------------------------------

struct ClassGenerator<'t> {
    // queue of classes that are to be generated
    // each tuple is (<class name>, <AST node representing class properties>)
    todos: VecDeque<(String, &'t Map<String, Value>)>
}

impl<'t> ClassGenerator<'t> {
    fn new() -> Self {
        Self { todos: VecDeque::new() }
    }
    
    /// Generates the classes corresponding to the given AST.
    /// Returns the classes as a string.
    fn generate(&mut self, ast: &'t Value) -> Result<String, Error> {
        // output builder
        let mut out = String::new();
        // root of the AST
        let root = ast.as_object().ok_or(fmt::Error)?;
        
        // generate the Root class. This corresponds to the base { ... } of the JSON.
        writeln!(&mut out, "public class Root\n{{")?; // class declaration {
        self.generate_properties(root, &mut out)?;    //   class properties
        writeln!(&mut out, "}}")?;                    // }
        
        // generate all other classes.
        while let Some((class, node)) = self.todos.pop_front() {
            writeln!(&mut out, "\npublic class {}\n{{", class)?; // class declaration {
            self.generate_properties(node, &mut out)?;           //   class properties
            writeln!(&mut out, "}}")?;                           // }
        }
        
        Ok(out)
    }
    
    /// Generate class properties
    /// eg. "public int id { get; set; }"
    fn generate_properties(&mut self, root: &'t Map<String, Value>, out: &mut String) -> Result<(), Error> {
        for entry in root {
            let class = self.find_type(entry);
            let varname = entry.0.to_ascii_lowercase();
            writeln!(out, "    public {} {} {{ get; set; }}", class, varname)?;
        }
        Ok(())
    }
    
    /// Find the concrete C# type corresponding to the JSON value
    /// given by the AST entry.
    /// Returns the concrete type as a CSharpType enum value.
    fn find_type(&mut self, entry: (&String, &'t Value)) -> CSharpType {
        let value = entry.1;
        match value {
            Value::String(_) => CSharpType::Primitive("string"),
            Value::Number(num) => match num {
                _ if num.is_i64() => CSharpType::Primitive("int"),
                _ if num.is_f64() => CSharpType::Primitive("float"),
                _ => CSharpType::Primitive("uint"),
            },
            Value::Bool(_) => CSharpType::Primitive("bool"),
            Value::Array(_) => CSharpType::Custom(self.flatten_type(entry)),
            Value::Object(obj) => {
                let class_name = Self::titlecase(entry.0);
                // enqueue an entry for this class to our TODOs so we
                // can later generate its class+properties at a later time.
                self.todos.push_back((class_name.clone(), obj));
                CSharpType::Custom(class_name)
            },
            Value::Null => unreachable!()
        }
    }
    
    /// Flatten the array indicated by the given entry in the AST.
    /// Returns the flattened type as a string.
    /// Example:
    ///     "nums": [ [ 14, -3, 8 ] ]  ->  "int[][]"
    fn flatten_type(&mut self, entry: (&String, &'t Value)) -> String {
        // Descends the AST until a concrete type is reached (anything other than an array),
        let mut bracket_count: usize = 0;
        let mut curr = entry.1;
        while let Value::Array(a) = curr {
            curr = a.first().unwrap();
            bracket_count += 1;
        }
        // concat the concrete name with the trailing array brackets
        self.find_type((entry.0, curr))
            .as_str()
            .to_owned()
            + "[]".repeat(bracket_count).as_str()
    }
    

    
    /// Returns a string identical to the given one except that the
    /// first letter of the returned string will be capitalized.
    /// eg. "weather" -> "Weather"
    fn titlecase(name: &str) -> String {
        let mut res = name.to_owned();
        if let Some(ch) = res.get_mut(0..1) {
            ch.make_ascii_uppercase();
        }
        res
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_weather() {
        let resources = env!("CARGO_MANIFEST_DIR").to_owned() + "/resources/";
        let input_path = resources.clone() + "weather.json";
        let answer_path = resources + "weather.cs";
        
        let ast =
            fs::read_to_string(input_path)
            .map(|weather_json|
                serde_json::from_str(&weather_json).unwrap()
            )
            .unwrap();
        
        let mut generator = ClassGenerator::new();
        let output = generator.generate(&ast).expect("Failed to generate classes.");
        
        let answer = fs::read_to_string(answer_path).unwrap();
        assert!(output.eq(&answer));
    }
}


