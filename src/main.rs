//! Generate C# classes from the given JSON
// TODO: handle name collisions?
use serde_json::{Value, Map};
use std::fmt::{Write, Error};
use std::collections::VecDeque;
use std::fmt;
use clap::{Arg, App};
use std::io::Read;

// ----------------------------------------------------------------------------
enum CSharpType {
    Primitive(&'static str),
    Custom(String),
}

impl fmt::Display for CSharpType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CSharpType::Primitive(typename) => write!(f, "{}", typename),
            CSharpType::Custom(typename) => write!(f, "{}", typename),
        }
    }
}

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
        std::fs::read_to_string(filename)
    } else {
        // read from stdin
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer).and_then(|_| Ok(buffer))
    };
    
    let json = input.expect("Failed to read JSON.");
    let ast: Value = serde_json::from_str(&json).unwrap();
    
    let mut generator = ClassGenerator::new();
    let out = generator.generate(&ast).expect("Failed to generate classes.");
    
    print!("{}", out);
}

// ----------------------------------------------------------------------------
struct ClassGenerator<'t> {
    todos: VecDeque<(String, &'t Map<String, Value>)>
}

impl<'t> ClassGenerator<'t> {
    fn new() -> Self {
        Self { todos: VecDeque::new() }
    }
    
    fn generate(&mut self, ast: &'t Value) -> Result<String, Error> {
        let mut out = String::new();
        let root = ast.as_object().ok_or(fmt::Error)?;
        
        writeln!(&mut out, "public class Root\n{{")?;
        self.generate_class(root, &mut out)?;
        writeln!(&mut out, "}}")?;
        
        while let Some((class, ast)) = self.todos.pop_front() {
            writeln!(&mut out, "\npublic class {}\n{{", class)?;
            self.generate_class(ast, &mut out)?;
            writeln!(&mut out, "}}")?;
        }
        
        Ok(out)
    }
    
    fn generate_class(&mut self, root: &'t Map<String, Value>, out: &mut String) -> Result<(), Error> {
        for entry in root {
            let classname = self.find_type(entry);
            let varname = entry.0.to_ascii_lowercase();
            writeln!(out, "    public {} {} {{ get; set; }}", classname, varname)?;
        }
        Ok(())
    }
    
    fn collapse_type(&mut self, entry: (&String, &'t Value)) -> String {
        let mut brackets = String::new();
        let mut curr = entry.1;
        while let Value::Array(a) = curr {
            curr = a.first().unwrap();
            write!(&mut brackets, "[]").unwrap();
        }
        self.find_type((entry.0, curr)).as_str().to_owned() + brackets.as_str()
    }
    
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
            Value::Array(_) => CSharpType::Custom(self.collapse_type(entry)),
            Value::Object(obj) => {
                let classname = Self::titlecase(entry.0);
                self.todos.push_back((classname.clone(), obj)); // add to todos
                CSharpType::Custom(classname)
            },
            Value::Null => unreachable!()
        }
    }
    
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
        
        let ast = fs::read_to_string(input_path)
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


