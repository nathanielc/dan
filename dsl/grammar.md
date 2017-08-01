# Define the Jim DSL grammar


```
digit               = "0" … "9" .
ascii_letter        = "A" … "Z" | "a" … "z" .
letter              = ascii_letter | "_" .
word                = ( letter ) { letter | digit } .
time                = ( digit ) ":" ( digit ) ( "AM" | "PM" ) .


Program           = { ProgramStatement | BlockStatement } .
Block             = "{" { BlockStatement } "}"  | BlockStatement .
ProgramStatement  = SceneStatement .
BlockStatement    = SetStatement | GetStatement | VarStatement | AtStatement | WhenStatement .
SetStatement      = "set" PathMatch Value .
VarStatement      = "var" word "=" GetStatement .
GetStatement      = "get" PathMatch .
SceneStatement    = "scene" word Block .
AtStatement       = "at" Time Action word .
Time              = { digit } ":" { digit } ( "AM" | "PM" )
Action            = ( "start" | "stop" )
WhenStatement     = "when" PathMatch "is" Value "wait" duration Block  | "when" PathMatch "is" Value Block .
PathMatch         = "$" | { ( word | "*" ) "/" } ( word | "*" ) .
```
