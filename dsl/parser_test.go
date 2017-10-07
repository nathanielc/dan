package dsl_test

import (
	"testing"
	"time"

	"github.com/google/go-cmp/cmp"
	"github.com/nathanielc/jim/dsl"
)

func TestParser(t *testing.T) {
	testCases := map[string]struct {
		input string
		ast   dsl.AST
	}{
		"set_statement": {
			input: "set masterbedroom/lights off",
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.SetStatementNode{
						Position: dsl.Position{Line: 1, Char: 1},
						DeviceMatch: &dsl.PathMatchNode{
							Position: dsl.Position{Line: 1, Char: 5},
							Path:     "masterbedroom/lights",
						},
						Value: &dsl.ValueNode{
							Position: dsl.Position{Line: 1, Char: 26},
							Value:    "off",
							Literal:  "off",
						},
					},
				},
			},
		},
		"var_statement": {
			input: "var x = get masterbedroom/lights",
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.VarStatementNode{
						Position: dsl.Position{Line: 1, Char: 1},
						Identifier: dsl.Token{
							Pos:   dsl.Position{Line: 1, Char: 5},
							Type:  dsl.TokenWord,
							Value: "x",
						},
						Get: &dsl.GetStatementNode{
							Position: dsl.Position{Line: 1, Char: 9},
							Path: &dsl.PathNode{
								Position: dsl.Position{Line: 1, Char: 13},
								Path:     "masterbedroom/lights",
							},
						},
					},
				},
			},
		},
		"at_statement": {
			input: "at 10:00 AM start workout",
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.AtStatementNode{
						Position: dsl.Position{Line: 1, Char: 1},
						Time: &dsl.TimeNode{
							Position: dsl.Position{Line: 1, Char: 4},
							Hour:     10,
							Minute:   0,
							AM:       true,
							Literal:  "10:00",
						},
						Block: &dsl.BlockNode{
							Position: dsl.Position{Line: 1, Char: 13},
							Statements: []dsl.Node{
								&dsl.StartStatementNode{
									Position: dsl.Position{Line: 1, Char: 13},
									Identifier: dsl.Token{
										Pos:   dsl.Position{Line: 1, Char: 19},
										Type:  dsl.TokenWord,
										Value: "workout",
									},
								},
							},
						},
					},
				},
			},
		},
		"when_statement": {
			input: `
when
	*/doors is unlocked
wait 5m
	set $ locked
`,
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.WhenStatementNode{
						Position: dsl.Position{Line: 2, Char: 1},
						Path: &dsl.PathMatchNode{
							Position: dsl.Position{Line: 3, Char: 2},
							Path:     "*/doors",
						},
						IsValue: &dsl.ValueNode{
							Position: dsl.Position{Line: 3, Char: 13},
							Value:    "unlocked",
							Literal:  "unlocked",
						},
						WaitDuration: &dsl.DurationNode{
							Position: dsl.Position{Line: 4, Char: 6},
							Duration: 5 * time.Minute,
							Literal:  "5m",
						},
						Block: &dsl.BlockNode{
							Position: dsl.Position{Line: 5, Char: 2},
							Statements: []dsl.Node{
								&dsl.SetStatementNode{
									Position: dsl.Position{Line: 5, Char: 2},
									DeviceMatch: &dsl.PathMatchNode{
										Position: dsl.Position{Line: 5, Char: 6},
										Path:     "$",
									},
									Value: &dsl.ValueNode{
										Position: dsl.Position{Line: 5, Char: 8},
										Value:    "locked",
										Literal:  "locked",
									},
								},
							},
						},
					},
				},
			},
		},
		"scene_statement": {
			input: `
scene nightime {
	set */light off
	set */door locked
	set porch/light on

	when
		*/door is unlocked
	wait 5m
		 set $ locked
}
`,
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.SceneStatementNode{
						Position: dsl.Position{Line: 2, Char: 1},
						Identifier: dsl.Token{
							Pos:   dsl.Position{Line: 2, Char: 7},
							Type:  dsl.TokenWord,
							Value: "nightime",
						},
						Block: &dsl.BlockNode{
							Position: dsl.Position{Line: 2, Char: 16},
							Statements: []dsl.Node{
								&dsl.SetStatementNode{
									Position: dsl.Position{Line: 3, Char: 2},
									DeviceMatch: &dsl.PathMatchNode{
										Position: dsl.Position{Line: 3, Char: 6},
										Path:     "*/light",
									},
									Value: &dsl.ValueNode{
										Position: dsl.Position{Line: 3, Char: 14},
										Value:    "off",
										Literal:  "off",
									},
								},
								&dsl.SetStatementNode{
									Position: dsl.Position{Line: 4, Char: 2},
									DeviceMatch: &dsl.PathMatchNode{
										Position: dsl.Position{Line: 4, Char: 6},
										Path:     "*/door",
									},
									Value: &dsl.ValueNode{
										Position: dsl.Position{Line: 4, Char: 13},
										Value:    "locked",
										Literal:  "locked",
									},
								},
								&dsl.SetStatementNode{
									Position: dsl.Position{Line: 5, Char: 2},
									DeviceMatch: &dsl.PathMatchNode{
										Position: dsl.Position{Line: 5, Char: 6},
										Path:     "porch/light",
									},
									Value: &dsl.ValueNode{
										Position: dsl.Position{Line: 5, Char: 18},
										Value:    "on",
										Literal:  "on",
									},
								},
								&dsl.WhenStatementNode{
									Position: dsl.Position{Line: 7, Char: 2},
									Path: &dsl.PathMatchNode{
										Position: dsl.Position{Line: 8, Char: 3},
										Path:     "*/door",
									},
									IsValue: &dsl.ValueNode{
										Position: dsl.Position{Line: 8, Char: 13},
										Value:    "unlocked",
										Literal:  "unlocked",
									},
									WaitDuration: &dsl.DurationNode{
										Position: dsl.Position{Line: 9, Char: 7},
										Duration: 5 * time.Minute,
										Literal:  "5m",
									},
									Block: &dsl.BlockNode{
										Position: dsl.Position{Line: 10, Char: 4},
										Statements: []dsl.Node{
											&dsl.SetStatementNode{
												Position: dsl.Position{Line: 10, Char: 4},
												DeviceMatch: &dsl.PathMatchNode{
													Position: dsl.Position{Line: 10, Char: 8},
													Path:     "$",
												},
												Value: &dsl.ValueNode{
													Position: dsl.Position{Line: 10, Char: 10},
													Value:    "locked",
													Literal:  "locked",
												},
											},
										},
									},
								},
							},
						},
					},
				},
			},
		},
		"stop_statement": {
			input: "stop nightime",
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.StopStatementNode{
						Position: dsl.Position{Line: 1, Char: 1},
						Identifier: dsl.Token{
							Pos:   dsl.Position{Line: 1, Char: 6},
							Type:  dsl.TokenWord,
							Value: "nightime",
						},
					},
				},
			},
		},
		"start_statement": {
			input: "start nightime",
			ast: &dsl.ProgramNode{
				Position: dsl.Position{Line: 1, Char: 1},
				Statements: []dsl.Node{
					&dsl.StartStatementNode{
						Position: dsl.Position{Line: 1, Char: 1},
						Identifier: dsl.Token{
							Pos:   dsl.Position{Line: 1, Char: 7},
							Type:  dsl.TokenWord,
							Value: "nightime",
						},
					},
				},
			},
		},
	}
	for name, tc := range testCases {
		name := name
		tc := tc
		t.Run(name, func(t *testing.T) {
			got, err := dsl.Parse(tc.input)
			if err != nil {
				t.Fatal(err)
			}
			if !cmp.Equal(got, tc.ast) {
				t.Errorf("unexpected ast:\n%s", cmp.Diff(got, tc.ast))
			}
		})
	}
}
