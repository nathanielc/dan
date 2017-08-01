package dsl

import "time"

type Node interface {
	Pos() Position
}

type Position struct {
	Line int
	Char int
}

func (p Position) Pos() Position {
	return p
}

type AST Node

type ProgramNode struct {
	Position

	Statements []Node
}

type SetStatementNode struct {
	Position
	DeviceMatch *PathMatchNode
	Value       *ValueNode
}

type ValueNode struct {
	Position
	Value   string
	Literal string
}

type PathMatchNode struct {
	Position
	Path string
}

type SceneStatementNode struct {
	Position
	Identifier Token
	Block      *BlockNode
}

type BlockNode struct {
	Position
	Statements []Node
}

type VarStatementNode struct {
	Position
	Identifier Token
	Get        *GetStatementNode
}

type GetStatementNode struct {
	Position
	Path *PathMatchNode
}

type AtStatementNode struct {
	Position
	Time       *TimeNode
	Action     *ActionNode
	Identifier Token
}

type TimeNode struct {
	Position
	Hour    int
	Minute  int
	AM      bool
	Literal string
}

type ActionNode struct {
	Position
	Action string
}

type WhenStatementNode struct {
	Position
	Path         *PathMatchNode
	IsValue      *ValueNode
	WaitDuration *DurationNode
	Block        *BlockNode
}

type DurationNode struct {
	Position
	Duration time.Duration
	Literal  string
}
