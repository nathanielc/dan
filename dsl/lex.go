package dsl

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

type TokenType int

type Token struct {
	Pos   Position
	Type  TokenType
	Value string
}

const (
	TokenError TokenType = iota
	TokenEOF

	TokenList
	TokenSet
	TokenGet
	TokenVar
	TokenScene
	TokenAt
	TokenWhen
	TokenWait
	TokenIs
	TokenAM
	TokenPM
	TokenStart
	TokenStop

	TokenWord
	TokenString
	TokenNumber
	TokenDuration
	TokenTime
	TokenAsign
	TokenStar
	TokenPathSeparator
	TokenDollar

	TokenOpenBracket
	TokenCloseBracket
)

func (tt TokenType) String() string {
	switch tt {
	case TokenError:
		return "error"
	case TokenEOF:
		return "eof"
	case TokenList:
		return "list"
	case TokenSet:
		return "set"
	case TokenGet:
		return "get"
	case TokenVar:
		return "var"
	case TokenScene:
		return "scene"
	case TokenAt:
		return "at"
	case TokenWhen:
		return "when"
	case TokenWait:
		return "wait"
	case TokenIs:
		return "is"
	case TokenAM:
		return "am"
	case TokenPM:
		return "pm"
	case TokenStart:
		return "start"
	case TokenStop:
		return "stop"
	case TokenWord:
		return "word"
	case TokenString:
		return "string"
	case TokenNumber:
		return "number"
	case TokenDuration:
		return "duration"
	case TokenTime:
		return "time"
	case TokenAsign:
		return "asign"
	case TokenStar:
		return "star"
	case TokenPathSeparator:
		return "pathseparator"
	case TokenDollar:
		return "dollar"
	case TokenOpenBracket:
		return "openbracket"
	case TokenCloseBracket:
		return "closebracket"
	default:
		return strconv.Itoa(int(tt))
	}
}

var keywords = map[string]TokenType{
	"list":  TokenList,
	"set":   TokenSet,
	"get":   TokenGet,
	"var":   TokenVar,
	"scene": TokenScene,
	"at":    TokenAt,
	"start": TokenStart,
	"stop":  TokenStop,
	"when":  TokenWhen,
	"wait":  TokenWait,
	"is":    TokenIs,
	"AM":    TokenAM,
	"PM":    TokenPM,
}

type lexer struct {
	input string // the string being lexed

	pos   int // the current position of the input
	start int // the start of the current token
	width int // the width of the last read rune

	line int // the line number of the current token
	char int // the character number of the current token

	tokens chan Token // channel on which to emit tokens
}

func newLexer(input string) *lexer {
	return &lexer{
		input:  input,
		pos:    0,
		start:  0,
		width:  0,
		line:   1,
		char:   1,
		tokens: make(chan Token),
	}
}

func Lex(input string) <-chan Token {
	l := newLexer(input)
	go func() {
		defer close(l.tokens)
		for state := lexToken; state != nil; {
			state = state(l)
		}
	}()
	return l.tokens
}

type stateFn func(l *lexer) stateFn

const eof = -1

func (l *lexer) emit(t TokenType) {
	value := l.current()
	l.tokens <- Token{
		Pos:   l.position(),
		Type:  t,
		Value: value,
	}
	l.updatePosCounters()
}

// ignore skips over the pending input before this point.
func (l *lexer) ignore() {
	l.updatePosCounters()
}

func (l *lexer) updatePosCounters() {
	value := l.current()
	// Update position counters
	l.start = l.pos

	// Count lines
	lastLine := 0
	for {
		i := strings.IndexRune(value[lastLine:], '\n')
		if i == -1 {
			break
		}
		lastLine += i + 1
		l.line++
		l.char = 1
	}
	l.char += len(value) - lastLine
}

func (l *lexer) position() Position {
	return Position{
		Line: l.line,
		Char: l.char,
	}
}

func (l *lexer) current() string {
	return l.input[l.start:l.pos]
}

func (l *lexer) next() rune {
	if l.pos >= len(l.input) {
		l.width = 0
		return eof
	}
	var r rune
	r, l.width = utf8.DecodeRuneInString(l.input[l.pos:])
	l.pos += l.width
	return r
}

//Backup the lexer to the previous rune
func (l *lexer) backup() {
	l.pos -= l.width
}

// peek returns but does not consume the next rune in the input.
func (l *lexer) peek() rune {
	r := l.next()
	l.backup()
	return r
}

// error emits an error token with the err and returns the terminal state.
func (l *lexer) error(err error) stateFn {
	l.tokens <- Token{Pos: l.position(), Type: TokenError, Value: err.Error()}
	return nil
}

// errorf emits an error token with the formatted arguments and returns the terminal state.
func (l *lexer) errorf(format string, args ...interface{}) stateFn {
	l.tokens <- Token{Pos: l.position(), Type: TokenError, Value: fmt.Sprintf(format, args...)}
	return nil
}

// ignore a contiguous block of spaces.
func (l *lexer) ignoreSpace() {
	for unicode.IsSpace(l.next()) {
		l.ignore()
	}
	l.backup()
}

/////////////////////////////
// Lex stateFn

// lexToken is the top level state
func lexToken(l *lexer) stateFn {
	for {
		switch r := l.next(); {
		case unicode.IsLetter(r):
			return lexWordOrKeyword
		case unicode.IsDigit(r):
			return lexNumberOrTimeOrDuration
		case r == '/':
			l.emit(TokenPathSeparator)
		case r == '*':
			l.emit(TokenStar)
		case r == '=':
			l.emit(TokenAsign)
		case r == '$':
			l.emit(TokenDollar)
		case r == '{':
			l.emit(TokenOpenBracket)
		case r == '}':
			l.emit(TokenCloseBracket)
		case r == '\'' || r == '"':
			return lexEscapedQuotedString(r)
		case unicode.IsSpace(r):
			l.ignore()
		case r == eof:
			l.emit(TokenEOF)
			return nil
		default:
			return l.errorf("unexpected token %v", r)
		}
	}
}

func lexWordOrKeyword(l *lexer) stateFn {
	for {
		switch r := l.next(); {
		case isValidIdent(r):
			// absorb
		default:
			l.backup()
			if typ, ok := keywords[l.current()]; ok {
				l.emit(typ)
				return lexToken
			}
			l.emit(TokenWord)
			return lexToken
		}
	}
}

func lexEscapedQuotedString(quote rune) stateFn {
	return func(l *lexer) stateFn {
		for {
			switch r := l.next(); {
			case r == '\\':
				if l.peek() == quote {
					l.next()
				}
			case r == quote:
				l.emit(TokenString)
				return lexToken
			}
		}
	}
}

// isValidIdent reports whether r is either a letter or a digit
func isValidIdent(r rune) bool {
	return unicode.IsDigit(r) || unicode.IsLetter(r) || r == '_'
}

const durationUnits = "usmh"

func isDurUnit(r rune) bool {
	return strings.IndexRune(durationUnits, r) != -1
}

func lexNumberOrTimeOrDuration(l *lexer) stateFn {
	for {
		switch r := l.next(); {
		case unicode.IsDigit(r):
			//absorb
		case r == '.':
			return lexNumberDigits
		case r == ':':
			return lexTimeDigits
		case isDurUnit(r):
			if r == 'm' && l.peek() == 's' {
				l.next()
			}
			l.emit(TokenDuration)
			return lexToken
		default:
			l.backup()
			l.emit(TokenNumber)
			return lexToken
		}
	}
}

func lexNumberDigits(l *lexer) stateFn {
	for {
		switch r := l.next(); {
		case unicode.IsDigit(r):
			//absorb
		default:
			l.backup()
			l.emit(TokenNumber)
			return lexToken
		}
	}
}

func lexTimeDigits(l *lexer) stateFn {
	for {
		switch r := l.next(); {
		case unicode.IsDigit(r):
			//absorb
		default:
			l.backup()
			l.emit(TokenTime)
			// Ignore space between time and AM|PM.
			l.ignoreSpace()
			return lexAMPM
		}
	}
}

func lexAMPM(l *lexer) stateFn {
	for {
		switch r := l.next(); r {
		case 'A', 'P':
			if l.next() != 'M' {
				return l.errorf("expected AM or PM")
			}

			if r == 'A' {
				l.emit(TokenAM)
			} else {
				l.emit(TokenPM)
			}
			return lexToken
		default:
			return l.errorf("expected AM or PM")
		}
	}
}
