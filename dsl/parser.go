package dsl

import (
	"bytes"
	"fmt"
	"path"
	"runtime"
	"strconv"
	"strings"
	"time"
)

func Parse(input string) (AST, error) {
	parser := newParser(Lex(input))
	return parser.Parse()
}

type parser struct {
	tokens <-chan Token

	lookahead [2]Token
	peekCount int
}

func newParser(tokens <-chan Token) *parser {
	return &parser{
		tokens: tokens,
	}
}

func (p *parser) Parse() (ast AST, err error) {
	// Parsing uses panics to bubble up errors
	defer p.recover(&err)

	ast = p.program()

	return
}

func (p *parser) nextToken() Token {
	return <-p.tokens
}

// next returns the next token.
func (p *parser) next() Token {
	if p.peekCount > 0 {
		p.peekCount--
	} else {
		p.lookahead[0] = p.nextToken()
	}
	return p.lookahead[p.peekCount]
}

// backup backs the input stream up one token.
func (p *parser) backup() {
	p.peekCount++
}

// peek returns but does not consume the next token.
func (p *parser) peek() Token {
	if p.peekCount > 0 {
		return p.lookahead[p.peekCount-1]
	}
	p.peekCount = 1
	p.lookahead[1] = p.lookahead[0]
	p.lookahead[0] = p.nextToken()
	return p.lookahead[0]
}

// errorf formats the error and terminates processing.
func (p *parser) errorf(format string, args ...interface{}) {
	format = fmt.Sprintf("parser: %s", format)
	panic(fmt.Errorf(format, args...))
}

// error terminates processing.
func (p *parser) error(err error) {
	p.errorf("%s", err)
}

// expect consumes the next token and guarantees it has the required type.
func (p *parser) expect(expected TokenType) Token {
	t := p.next()
	if t.Type != expected {
		p.unexpected(t, expected)
	}
	return t
}

// unexpected complains about the token and terminates processing.
func (p *parser) unexpected(tok Token, expected ...TokenType) {
	expectedStrs := make([]string, len(expected))
	for i := range expected {
		expectedStrs[i] = fmt.Sprintf("%q", expected[i])
	}
	expectedStr := strings.Join(expectedStrs, ",")
	p.errorf("unexpected token %q with value %q at line %d char %d, expected: %s", tok.Type, tok.Value, tok.Pos.Line, tok.Pos.Char, expectedStr)
}

// recover is the handler that turns panics into returns from the top level of Parse.
func (p *parser) recover(errp *error) {
	e := recover()
	if e != nil {
		if _, ok := e.(runtime.Error); ok {
			panic(e)
		}
		*errp = e.(error)
	}
	return
}

////////////////////////////////
// Grammar Production rules

var positionZero = Position{
	Line: 1,
	Char: 1,
}

func (p *parser) program() *ProgramNode {
	prog := &ProgramNode{
		Position: positionZero,
	}
	for {
		switch p.peek().Type {
		case TokenEOF:
			return prog
		case TokenScene:
			s := p.programStatement()
			prog.Statements = append(prog.Statements, s)
		default:
			s := p.blockStatement()
			prog.Statements = append(prog.Statements, s)
		}
	}
}

func (p *parser) programStatement() Node {
	return p.sceneStatement()
}

func (p *parser) blockStatement() Node {
	switch p.peek().Type {
	case TokenSet:
		return p.setStatement()
	case TokenGet:
		return p.getStatement()
	case TokenVar:
		return p.varStatement()
	case TokenAt:
		return p.atStatement()
	case TokenWhen:
		return p.whenStatement()
	default:
		p.unexpected(p.next(), TokenSet, TokenVar, TokenAt, TokenWhen)
		return nil
	}
}

func (p *parser) sceneStatement() *SceneStatementNode {
	s := p.expect(TokenScene)
	w := p.expect(TokenWord)
	b := p.block()
	return &SceneStatementNode{
		Position:   s.Pos,
		Identifier: w,
		Block:      b,
	}
}

func (p *parser) block() *BlockNode {
	b := &BlockNode{
		Position: p.peek().Pos,
	}
	if p.peek().Type == TokenOpenBracket {
		p.next()
		for p.peek().Type != TokenCloseBracket {
			s := p.blockStatement()
			b.Statements = append(b.Statements, s)
		}
		p.expect(TokenCloseBracket)
	} else {
		s := p.blockStatement()
		b.Statements = append(b.Statements, s)
	}
	return b
}

func (p *parser) setStatement() *SetStatementNode {
	t := p.expect(TokenSet)
	pm := p.pathMatch()
	v := p.value()
	return &SetStatementNode{
		Position:    t.Pos,
		DeviceMatch: pm,
		Value:       v,
	}
}

func (p *parser) pathMatch() *PathMatchNode {
	pm := &PathMatchNode{
		Position: p.peek().Pos,
		Path:     "",
	}
	for {
		switch p.peek().Type {
		case TokenDollar:
			p.next()
			pm.Path = "$"
			return pm
		case TokenStar, TokenWord:
			t := p.next()
			pm.Path = path.Join(pm.Path, t.Value)
		default:
			if pm.Path == "" {
				p.unexpected(p.next(), TokenStar, TokenWord)
				return nil
			}
			return pm
		}
		if p.peek().Type != TokenPathSeparator {
			return pm
		}
		p.next()
	}
}

func (p *parser) value() *ValueNode {
	switch p.peek().Type {
	case TokenWord, TokenNumber:
		t := p.next()
		return &ValueNode{
			Position: t.Pos,
			Value:    t.Value,
			Literal:  t.Value,
		}
	case TokenString:
		t := p.next()
		value := unescapeString(t.Value)
		return &ValueNode{
			Position: t.Pos,
			Value:    value,
			Literal:  t.Value,
		}
	default:
		p.unexpected(p.next(), TokenWord, TokenString, TokenNumber)
		return nil
	}
}

// unescapeString returns the quoted string with leading, trailing and escaped characters removed.
func unescapeString(txt string) string {
	literal := txt[1 : len(txt)-1]
	quote := txt[0]
	// Unescape quotes
	var buf bytes.Buffer
	buf.Grow(len(literal))
	last := 0
	for i := 0; i < len(literal)-1; i++ {
		if literal[i] == '\\' && literal[i+1] == quote {
			buf.Write([]byte(literal[last:i]))
			i++
			last = i
		}
	}
	buf.Write([]byte(literal[last:]))
	return buf.String()
}

func (p *parser) varStatement() *VarStatementNode {
	t := p.expect(TokenVar)
	w := p.expect(TokenWord)
	p.expect(TokenAsign)
	g := p.getStatement()
	return &VarStatementNode{
		Position:   t.Pos,
		Identifier: w,
		Get:        g,
	}
}

func (p *parser) getStatement() *GetStatementNode {
	t := p.expect(TokenGet)
	pm := p.pathMatch()
	return &GetStatementNode{
		Position: t.Pos,
		Path:     pm,
	}
}

func (p *parser) atStatement() *AtStatementNode {
	t := p.expect(TokenAt)
	tm := p.time()
	a := p.action()
	w := p.expect(TokenWord)
	return &AtStatementNode{
		Position:   t.Pos,
		Time:       tm,
		Action:     a,
		Identifier: w,
	}
}

func (p *parser) time() *TimeNode {
	t := p.expect(TokenTime)
	// Parse time literal
	parts := strings.Split(t.Value, ":")
	if len(parts) != 2 {
		p.errorf("unexpected time literal %q", t.Value)
		return nil
	}
	h, err := strconv.Atoi(parts[0])
	if err != nil {
		p.error(err)
		return nil
	}
	m, err := strconv.Atoi(parts[1])
	if err != nil {
		p.error(err)
		return nil
	}
	if h < 0 || h > 12 {
		p.errorf("hour must be between 0 and 12")
		return nil
	}
	if m < 0 || m > 59 {
		p.errorf("minute must be between 0 and 59")
		return nil
	}

	tm := &TimeNode{
		Position: t.Pos,
		Literal:  t.Value,
		Hour:     h,
		Minute:   m,
	}
	switch p.peek().Type {
	case TokenAM:
		p.next()
		tm.AM = true
	case TokenPM:
		p.next()
		tm.AM = false
	default:
		p.unexpected(p.next(), TokenAM, TokenPM)
		return nil
	}
	return tm
}

func (p *parser) action() *ActionNode {
	switch p.peek().Type {
	case TokenStart, TokenStop:
		t := p.next()
		return &ActionNode{
			Position: t.Pos,
			Action:   t.Value,
		}
	default:
		p.unexpected(p.next(), TokenStart, TokenStop)
		return nil
	}
}

func (p *parser) whenStatement() *WhenStatementNode {
	t := p.expect(TokenWhen)
	pm := p.pathMatch()
	p.expect(TokenIs)
	v := p.value()
	var d *DurationNode
	if p.peek().Type == TokenWait {
		p.expect(TokenWait)
		d = p.duration()
	}
	b := p.block()
	return &WhenStatementNode{
		Position:     t.Pos,
		Path:         pm,
		IsValue:      v,
		WaitDuration: d,
		Block:        b,
	}
}

func (p *parser) duration() *DurationNode {
	t := p.expect(TokenDuration)
	d, err := time.ParseDuration(t.Value)
	if err != nil {
		p.error(err)
		return nil
	}
	return &DurationNode{
		Position: t.Pos,
		Duration: d,
		Literal:  t.Value,
	}
}
