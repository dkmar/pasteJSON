> NOTE: if you actually need this utility, use a more proven tool like [quicktype](https://github.com/quicktype/quicktype)

Summary
---
Emulates the `Paste JSON as Classes` feature of Visual Studio.

Given either a file containing JSON or a JSON object on stdin, this program maps the JSON structure to C# classes and prints their class declarations.

USAGE:
    paste_json [file]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <file>    The file containing the JSON object

EXAMPLE USAGE:

        (1)   paste_json weather.json
        
        (2)   cat weather.json | paste_json
        

Example Results
---
```
{
  "menu": {
    "header": "SVG Viewer",
    "items": [
      {
        "id": "Open",
        "label": "Open"
      },
      {
        "id": "OpenNew",
        "label": "Open New"
      }
    ]
  }
}
```
```
public class Root
{
    public Menu menu { get; set; }
}

public class Menu
{
    public string header { get; set; }
    public Items[] items { get; set; }
}

public class Items
{
    public string id { get; set; }
    public string label { get; set; }
}
```
