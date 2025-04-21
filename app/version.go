package app

var (
	version        string
	go_version     string
	build_time     string
	commit_hash    string
	git_branch     string
	git_tree_state string
)

// Version returns the version of the binary
func Version() string {
	return version
}
