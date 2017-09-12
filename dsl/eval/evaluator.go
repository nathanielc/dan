package eval

import (
	"bytes"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/smartmqtt"
)

type Result interface {
	String() string
}
type Evaluator struct {
	c           smartmqtt.Client
	scenes      map[string]*sceneState
	globalScene *sceneState

	mu sync.Mutex
}

func New(c smartmqtt.Client) *Evaluator {
	return &Evaluator{
		c:           c,
		scenes:      make(map[string]*sceneState),
		globalScene: new(sceneState),
	}
}

func (e *Evaluator) Eval(ast dsl.AST) (Result, error) {
	return e.eval(e.globalScene, ast)
}
func (e *Evaluator) eval(scene *sceneState, node dsl.Node) (Result, error) {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.evalWithLock(scene, node)
}

func (e *Evaluator) evalWithLock(scene *sceneState, node dsl.Node) (Result, error) {
	switch n := node.(type) {
	case *dsl.ProgramNode:
		return e.evalNodeList(scene, n.Statements)
	case *dsl.SetStatementNode:
		return e.evalSet(n)
	case *dsl.GetStatementNode:
		return e.evalGet(n)
	case *dsl.WhenStatementNode:
		return e.evalWhen(scene, n)
	case *dsl.BlockNode:
		return e.evalNodeList(scene, n.Statements)
	case *dsl.AtStatementNode:
		return e.evalAt(scene, n)
	case *dsl.SceneStatementNode:
		return e.evalDefineScene(n)
	case *dsl.StartStatementNode:
		return e.evalStartScene(n)
	case *dsl.StopStatementNode:
		return e.evalStopScene(n)
	default:
		return nil, fmt.Errorf("unknown command %T", node)
	}
}
func (e *Evaluator) evalAt(scene *sceneState, n *dsl.AtStatementNode) (Result, error) {
	hour := n.Time.Hour
	if !n.Time.AM {
		hour += 12
	}

	cancel, err := scheduleDaily(hour, n.Time.Minute, func(time.Time) {
		e.eval(scene, n.Block)
	})
	if err != nil {
		return nil, err
	}
	scene.cancel = append(scene.cancel, cancel)
	return nil, nil
}

func (e *Evaluator) evalDefineScene(n *dsl.SceneStatementNode) (Result, error) {
	s := &sceneState{
		block: n.Block,
	}
	e.scenes[n.Identifier.Value] = s
	return nil, nil
}

func (e *Evaluator) evalStartScene(n *dsl.StartStatementNode) (Result, error) {
	name := n.Identifier.Value
	s, ok := e.scenes[name]
	if !ok {
		return nil, fmt.Errorf("unknown scene %q", name)
	}
	return e.evalWithLock(s, s.block)
}
func (e *Evaluator) evalStopScene(n *dsl.StopStatementNode) (Result, error) {
	name := n.Identifier.Value
	s, ok := e.scenes[name]
	if !ok {
		return nil, fmt.Errorf("unknown scene %q", name)
	}
	s.Stop()
	delete(e.scenes, name)
	return nil, nil
}

func (e *Evaluator) evalNodeList(scene *sceneState, ss []dsl.Node) (Result, error) {
	listResult := make(listResult, len(ss))
	for i, s := range ss {
		r, err := e.evalWithLock(scene, s)
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

func (e *Evaluator) evalWhen(scene *sceneState, w *dsl.WhenStatementNode) (Result, error) {
	toplevel, topic, err := splitPathMatch(w.Path.Path)
	if err != nil {
		return nil, err
	}
	if cancel, err := e.c.When(toplevel, topic, w.IsValue.Value, func() {
		if w.WaitDuration != nil {
			time.AfterFunc(w.WaitDuration.Duration, func() { e.eval(scene, w.Block) })
		} else {
			e.eval(scene, w.Block)
		}
	}); err != nil {
		return nil, err
	} else {
		scene.cancel = append(scene.cancel, cancel)
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

type sceneState struct {
	block  *dsl.BlockNode
	cancel []func()
}

func (s *sceneState) Stop() {
	for _, c := range s.cancel {
		c()
	}
}
