package agent_tool

import "context"

type Tool interface {
	Name() string
	Desc() string
	ParamSchema() string
	Run(ctx context.Context, param string) (string, error)
}
