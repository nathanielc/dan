package repl

import (
	"bytes"
	"fmt"
	"strings"
	"time"

	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/smartmqtt"
)

type Result interface {
	String() string
}

type Evaluator struct {
	c smartmqtt.Client
}

func NewEvaluator(smart smartmqtt.Client) *Evaluator {
	return &Evaluator{
		c: smart,
	}
}

func (e *Evaluator) Eval(ast dsl.AST) (Result, error) {
	return e.eval(ast)
}

func (e *Evaluator) eval(node dsl.Node) (Result, error) {
	switch n := node.(type) {
	case *dsl.ProgramNode:
		return e.evalNodeList(n.Statements)
	case *dsl.SetStatementNode:
		return e.evalSet(n)
	case *dsl.GetStatementNode:
		return e.evalGet(n)
	case *dsl.WhenStatementNode:
		return e.evalWhen(n)
	case *dsl.BlockNode:
		return e.evalNodeList(n.Statements)
	default:
		return nil, fmt.Errorf("unknown command %T", node)
	}
}

func (e *Evaluator) evalNodeList(ss []dsl.Node) (Result, error) {
	listResult := make(listResult, len(ss))
	for i, s := range ss {
		r, err := e.eval(s)
		if err != nil {
			return nil, err
		}
		listResult[i] = r
	}
	return listResult, nil
}

func (e *Evaluator) evalSet(s *dsl.SetStatementNode) (Result, error) {
	toplevel, topic, err := splitPathMatch(s.DeviceMatch.Path)
	if err != nil {
		return nil, err
	}
	return nil, e.c.Set(toplevel, topic, s.Value.Value)
}

func (e *Evaluator) evalGet(g *dsl.GetStatementNode) (Result, error) {
	toplevel, topic, err := splitPathMatch(g.Path.Path)
	if err != nil {
		return nil, err
	}
	v, err := e.c.Get(toplevel, topic)
	if err != nil {
		return nil, err
	}
	return result{v: v.Value}, nil
}

func (e *Evaluator) evalWhen(w *dsl.WhenStatementNode) (Result, error) {
	toplevel, topic, err := splitPathMatch(w.Path.Path)
	if err != nil {
		return nil, err
	}
	if err := e.c.When(toplevel, topic, w.IsValue.Value, func() {
		if w.WaitDuration != nil {
			time.AfterFunc(w.WaitDuration.Duration, func() { e.eval(w.Block) })
		} else {
			e.eval(w.Block)
		}
	}); err != nil {
		return nil, err
	}
	return nil, nil
}

type result struct {
	v interface{}
}

func (r result) String() string {
	return fmt.Sprintf("%v", r.v)
}

func splitPathMatch(p string) (toplevel, device string, err error) {
	i := strings.IndexRune(p, '/')
	if i < 0 {
		err = fmt.Errorf("invalid path %q", p)
		return
	}
	toplevel = p[:i]
	device = p[i+1:]

	return
}

type listResult []Result

func (l listResult) String() string {
	var buf bytes.Buffer
	for _, r := range l {
		if r != nil {
			buf.WriteString(r.String())
			buf.WriteByte('\n')
		}
	}
	return buf.String()
}
