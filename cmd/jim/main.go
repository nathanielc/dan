package main

import (
	"bufio"
	"fmt"
	"log"
	"os"

	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/dsl/repl"
	"github.com/nathanielc/jim/smartmqtt"
)

func main() {

	server, err := smartmqtt.New()
	if err != nil {
		log.Fatal(err)
	}

	scanner := bufio.NewScanner(os.Stdin)
	e := repl.NewEvaluator(server)
	for scanner.Scan() {
		ast, err := dsl.Parse(scanner.Text())
		r, err := e.Eval(ast)
		if err != nil {
			fmt.Println("E", err)
			continue
		}
		if r != nil {
			fmt.Println(r.String())
		}
	}
}
