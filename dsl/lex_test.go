package dsl_test

import (
	"testing"

	"github.com/nathanielc/jim/dsl"
)

func TestLexer(t *testing.T) {
	testCases := map[string]struct {
		input  string
		tokens []dsl.Token
	}{
		"keyword-set": {
			input: "set",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenSet,
					Value: "set",
				},
			},
		},
		"keyword-get": {
			input: "get",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenGet,
					Value: "get",
				},
			},
		},
		"path": {
			input: "p0/p1/p2",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenWord,
					Value: "p0",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 3},
					Type:  dsl.TokenPathSeparator,
					Value: "/",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 4},
					Type:  dsl.TokenWord,
					Value: "p1",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 6},
					Type:  dsl.TokenPathSeparator,
					Value: "/",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 7},
					Type:  dsl.TokenWord,
					Value: "p2",
				},
			},
		},
		"star-path": {
			input: "*/p1/*",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenStar,
					Value: "*",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 2},
					Type:  dsl.TokenPathSeparator,
					Value: "/",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 3},
					Type:  dsl.TokenWord,
					Value: "p1",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 5},
					Type:  dsl.TokenPathSeparator,
					Value: "/",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 6},
					Type:  dsl.TokenStar,
					Value: "*",
				},
			},
		},
		"integer": {
			input: "42",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenNumber,
					Value: "42",
				},
			},
		},
		"decimal": {
			input: "42.0",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenNumber,
					Value: "42.0",
				},
			},
		},
		"decimal-zero": {
			input: "0.0",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenNumber,
					Value: "0.0",
				},
			},
		},
		"decimal-tail": {
			input: "1.",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenNumber,
					Value: "1.",
				},
			},
		},
		"time-am": {
			input: "2:49 AM",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenTime,
					Value: "2:49",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 6},
					Type:  dsl.TokenAM,
					Value: "AM",
				},
			},
		},
		"time-pm": {
			input: "2:49 PM",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenTime,
					Value: "2:49",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 6},
					Type:  dsl.TokenPM,
					Value: "PM",
				},
			},
		},
		"time-pm-nospace": {
			input: "2:49PM",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenTime,
					Value: "2:49",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 5},
					Type:  dsl.TokenPM,
					Value: "PM",
				},
			},
		},
		"number-time": {
			input: "42 52:49 PM",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenNumber,
					Value: "42",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 4},
					Type:  dsl.TokenTime,
					Value: "52:49",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 10},
					Type:  dsl.TokenPM,
					Value: "PM",
				},
			},
		},
		"duration-0": {
			input: "5h",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenDuration,
					Value: "5h",
				},
			},
		},
		"invalid-duration-0": {
			input: "4.5h",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenNumber,
					Value: "4.5",
				},
				{
					Pos:   dsl.Position{Line: 1, Char: 4},
					Type:  dsl.TokenWord,
					Value: "h",
				},
			},
		},
		"dollar": {
			input: "$",
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenDollar,
					Value: "$",
				},
			},
		},
		"str-single": {
			input: `'single qout\'d string'`,
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenString,
					Value: `'single qout\'d string'`,
				},
			},
		},
		"str-double": {
			input: `'double qout\'d string'`,
			tokens: []dsl.Token{
				{
					Pos:   dsl.Position{Line: 1, Char: 1},
					Type:  dsl.TokenString,
					Value: `'double qout\'d string'`,
				},
			},
		},
	}

	for name, tc := range testCases {
		name := name
		tc := tc
		t.Run(name, func(t *testing.T) {
			tokens := dsl.Lex(tc.input)
			// Cheat an create fake EOF token
			expTokens := append(tc.tokens, dsl.Token{Type: dsl.TokenEOF})
			i := 0
			for got := range tokens {
				if i >= len(expTokens) {
					t.Fatalf("unexpected number of tokens: got %d exp %d", i+1, len(expTokens))
				}
				exp := expTokens[i]
				if exp.Type == dsl.TokenEOF {
					// Cheat an populate fake EOF token with correct position
					exp.Pos = got.Pos
				}
				if got != exp {
					t.Fatalf("unexpected %dth token: got %+v exp %+v", i, got, exp)
				}
				i++
			}
			if got, exp := i, len(expTokens); got != exp {
				t.Errorf("unexpected number of tokens: got %d exp %d", got, exp)
			}
		})
	}
}
