package runner

import (
	"context"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"kwatch/config"
)

func TestResultHistory_Add(t *testing.T) {
	h := &ResultHistory{}
	r1 := CommandResult{Command: "a", Timestamp: time.Now()}
	r2 := CommandResult{Command: "b", Timestamp: time.Now().Add(time.Second)}

	h.Add(r1)
	h.Add(r2)

	if got := len(h.Results); got != 2 {
		t.Errorf("len(Results) = %d, want 2", got)
	}
}

func TestResultHistory_GetLatest(t *testing.T) {
	now := time.Now()
	h := &ResultHistory{
		Results: []CommandResult{
			{Command: "npx tsc", Timestamp: now},
			{Command: "npx eslint", Timestamp: now.Add(time.Second)},
			{Command: "npx tsc", Timestamp: now.Add(2 * time.Second)},
		},
	}

	latest := h.GetLatest()
	if len(latest) != 2 {
		t.Errorf("GetLatest() returned %d types, want 2", len(latest))
	}
	ts, ok := latest[TypescriptCheck]
	if !ok {
		t.Fatal("TypescriptCheck missing from GetLatest()")
	}
	if !ts.Timestamp.Equal(now.Add(2 * time.Second)) {
		t.Errorf("TypescriptCheck latest timestamp = %v, want %v", ts.Timestamp, now.Add(2*time.Second))
	}
}

func TestResultHistory_GetAll(t *testing.T) {
	h := &ResultHistory{
		Results: []CommandResult{
			{Command: "a", IssueCount: 1},
			{Command: "b", IssueCount: 2},
		},
	}

	all := h.GetAll()
	if len(all) != 2 {
		t.Errorf("GetAll() length = %d, want 2", len(all))
	}

	// Verify it returns a copy (mutations don't affect original)
	all[0].Command = "modified"
	if h.Results[0].Command != "a" {
		t.Error("GetAll() did not return a copy")
	}
}

func TestResultHistory_Clear(t *testing.T) {
	h := &ResultHistory{
		Results: []CommandResult{
			{Command: "a"},
			{Command: "b"},
		},
	}
	h.Clear()
	if len(h.Results) != 0 {
		t.Errorf("after Clear(): len(Results) = %d, want 0", len(h.Results))
	}
}

func TestResultHistory_ConcurrentAccess(t *testing.T) {
	h := &ResultHistory{}

	var wg sync.WaitGroup
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			h.Add(CommandResult{Command: "concurrent"})
		}()
	}
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_ = h.GetAll()
			_ = h.GetLatest()
		}()
	}
	wg.Wait()

	if got := len(h.Results); got != 10 {
		t.Errorf("after concurrent adds: len(Results) = %d, want 10", got)
	}
}

func TestGetCommandType(t *testing.T) {
	tests := []struct {
		command string
		want    CommandType
	}{
		{"npx tsc --noEmit", TypescriptCheck},
		{"npx eslint .", LintCheck},
		{"npm test", TestRunner},
		{"some security scan", SecurityCheck},
		{"unknown tool", CommandType("unknown tool")},
		{"", CommandType("")},
	}

	for _, tt := range tests {
		t.Run(tt.command, func(t *testing.T) {
			if got := getCommandType(tt.command); got != tt.want {
				t.Errorf("getCommandType(%q) = %v, want %v", tt.command, got, tt.want)
			}
		})
	}
}

func TestNewRunner(t *testing.T) {
	cfg := RunnerConfig{
		DefaultTimeout: 5 * time.Second,
		MaxParallel:    2,
		WorkingDir:     ".",
	}
	kwatchConfig := DefaultKwatchConfig()

	r := NewRunner(cfg, kwatchConfig)
	if r == nil {
		t.Fatal("NewRunner() returned nil")
	}
	if r.config.DefaultTimeout != 5*time.Second {
		t.Errorf("config.DefaultTimeout = %v, want 5s", r.config.DefaultTimeout)
	}
	if r.history == nil {
		t.Error("history is nil")
	}
	if r.parser == nil {
		t.Error("parser is nil")
	}
}

func TestNewRunner_NilKwatchConfig(t *testing.T) {
	// Should still construct; falls back to hardcoded defaults in getDefaultCommands
	r := NewRunner(RunnerConfig{}, nil)
	if r == nil {
		t.Fatal("NewRunner() returned nil")
	}
	cmds := r.getDefaultCommands()
	if len(cmds) != 3 {
		t.Errorf("getDefaultCommands() with nil kwatchConfig: got %d, want 3", len(cmds))
	}
}

func TestRunner_GetLatestResults(t *testing.T) {
	r := NewRunner(RunnerConfig{}, nil)
	r.history.Add(CommandResult{Command: "npx tsc", Passed: true})

	got := r.GetLatestResults()
	if _, ok := got[TypescriptCheck]; !ok {
		t.Error("GetLatestResults() missing TypescriptCheck")
	}
}

func TestRunner_GetHistory(t *testing.T) {
	r := NewRunner(RunnerConfig{}, nil)
	r.history.Add(CommandResult{Command: "npx tsc"})
	r.history.Add(CommandResult{Command: "npx eslint"})

	h := r.GetHistory()
	if len(h) != 2 {
		t.Errorf("GetHistory() length = %d, want 2", len(h))
	}
}

func TestRunner_ClearHistory(t *testing.T) {
	r := NewRunner(RunnerConfig{}, nil)
	r.history.Add(CommandResult{Command: "npx tsc"})
	r.ClearHistory()
	if got := len(r.GetHistory()); got != 0 {
		t.Errorf("after ClearHistory(): len(GetHistory()) = %d, want 0", got)
	}
}

func TestRunner_getDefaultCommands_WithKwatchConfig(t *testing.T) {
	cfg := &config.Config{
		DefaultTimeout: "30s",
		MaxParallel:    2,
		Commands: map[string]config.Command{
			"typescript": {Command: "tsc", Args: []string{"--noEmit"}, Timeout: "30s", Enabled: true},
			"lint":       {Command: "eslint", Args: []string{"."}, Timeout: "30s", Enabled: true},
			"test":       {Command: "npm", Args: []string{"test"}, Timeout: "60s", Enabled: true},
			"disabled":   {Command: "x", Args: nil, Timeout: "10s", Enabled: false},
		},
	}
	r := NewRunner(RunnerConfig{}, cfg)
	cmds := r.getDefaultCommands()
	if len(cmds) != 3 {
		t.Errorf("getDefaultCommands() with kwatchConfig: got %d enabled, want 3", len(cmds))
	}
}

func TestRunner_getDefaultCommands_CustomCommandType(t *testing.T) {
	cfg := &config.Config{
		DefaultTimeout: "30s",
		MaxParallel:    2,
		Commands: map[string]config.Command{
			"typescript": {Command: "tsc", Timeout: "30s", Enabled: true},
			"mycustom":   {Command: "echo", Args: []string{"hi"}, Timeout: "5s", Enabled: true},
		},
	}
	r := NewRunner(RunnerConfig{}, cfg)
	cmds := r.getDefaultCommands()
	if _, ok := cmds[CommandType("mycustom")]; !ok {
		t.Error("custom command name not preserved as CommandType")
	}
}

func TestRunner_FormatCompactStatus(t *testing.T) {
	tests := []struct {
		name    string
		results map[CommandType]CommandResult
		want    string
	}{
		{
			name:    "empty results",
			results: map[CommandType]CommandResult{},
			want:    "",
		},
		{
			name: "all passed with tests counts",
			results: map[CommandType]CommandResult{
				TypescriptCheck: {Passed: true, IssueCount: 0},
				LintCheck:       {Passed: true, IssueCount: 0},
				TestRunner:      {Passed: true, TotalTests: 5, PassedTests: 5},
			},
			want: "TSC:✓0 LINT:✓0 TEST:✓5/5",
		},
		{
			name: "all passed simple",
			results: map[CommandType]CommandResult{
				TypescriptCheck: {Passed: true, IssueCount: 0},
				LintCheck:       {Passed: true, IssueCount: 0},
				TestRunner:      {Passed: true, IssueCount: 0},
			},
			want: "TSC:✓0 LINT:✓0 TEST:✓0",
		},
		{
			name: "typescript with issues",
			results: map[CommandType]CommandResult{
				TypescriptCheck: {Passed: false, IssueCount: 3, FileCount: 2},
				LintCheck:       {Passed: true, IssueCount: 0},
				TestRunner:      {Passed: true, IssueCount: 0},
			},
			want: "TSC:✗3/2 LINT:✓0 TEST:✓0",
		},
		{
			name: "lint failures",
			results: map[CommandType]CommandResult{
				TypescriptCheck: {Passed: true, IssueCount: 0},
				LintCheck:       {Passed: false, IssueCount: 5, FileCount: 3},
				TestRunner:      {Passed: true, IssueCount: 0},
			},
			want: "TSC:✓0 LINT:✗5/3 TEST:✓0",
		},
		{
			name: "test failures",
			results: map[CommandType]CommandResult{
				TypescriptCheck: {Passed: true, IssueCount: 0},
				LintCheck:       {Passed: true, IssueCount: 0},
				TestRunner:      {Passed: false, TotalTests: 10, PassedTests: 7, FailedTests: 3},
			},
			want: "TSC:✓0 LINT:✓0 TEST:✗7/10",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := FormatCompactStatus(tt.results)
			if got != tt.want {
				t.Errorf("FormatCompactStatus() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestRunner_ParseCommandOutput_DispatchesByType(t *testing.T) {
	r := NewRunner(RunnerConfig{}, nil)

	// TypeScript dispatch
	passed, _ := r.parseCommandOutput(TypescriptCheck, "")
	if !passed {
		t.Error("parseCommandOutput(TypescriptCheck, \"\") should pass")
	}

	// Lint dispatch
	passed, _ = r.parseCommandOutput(LintCheck, "")
	if !passed {
		t.Error("parseCommandOutput(LintCheck, \"\") should pass")
	}

	// Unknown type → generic
	passed, _ = r.parseCommandOutput(CommandType("unknown"), "ok")
	if !passed {
		t.Error("parseCommandOutput(unknown, \"ok\") should pass via generic")
	}
}

func TestRunner_extractFileCount(t *testing.T) {
	r := NewRunner(RunnerConfig{}, nil)

	tests := []struct {
		name   string
		output string
		want   int
	}{
		{
			name:   "empty output",
			output: "",
			want:   0,
		},
		{
			name:   "no files",
			output: "no files here",
			want:   0,
		},
		{
			name:   "single .ts file",
			output: "/path/to/file.ts\n  1:1  error  msg",
			want:   1,
		},
		{
			name:   "multiple unique files",
			output: "/path/to/a.ts\n  1:1  error\n/path/to/b.ts\n  1:1  error\n/path/to/a.ts\n  2:2  error",
			want:   2,
		},
		{
			name:   "mix of extensions",
			output: "./a.js\n./b.tsx\n./c.jsx\n./d.go",
			want:   3, // only .ts/.js/.tsx/.jsx counted
		},
		{
			name:   "relative paths",
			output: "./src/foo.ts\n  1:1  error",
			want:   1,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := r.extractFileCount(tt.output)
			if got != tt.want {
				t.Errorf("extractFileCount() = %d, want %d", got, tt.want)
			}
		})
	}
}

func TestRunner_RunCommand_Success(t *testing.T) {
	cfg := RunnerConfig{
		DefaultTimeout: 5 * time.Second,
		MaxParallel:    1,
		WorkingDir:     ".",
	}
	r := NewRunner(cfg, nil)

	// Use a command that's universally available and exits 0
	result := r.RunCommand(context.Background(), Command{
		Type:    TypescriptCheck,
		Command: "true",
		Args:    []string{},
		Timeout: 2 * time.Second,
	})

	if !result.Passed {
		t.Errorf("RunCommand with `true`: Passed = false, Error = %q, Output = %q", result.Error, result.Output)
	}
	if result.Duration < 0 {
		t.Errorf("RunCommand: Duration = %v, expected >= 0", result.Duration)
	}
	if result.Timestamp.IsZero() {
		t.Error("RunCommand: Timestamp is zero")
	}
}

func TestRunner_RunCommand_Failure(t *testing.T) {
	cfg := RunnerConfig{
		DefaultTimeout: 5 * time.Second,
		WorkingDir:     ".",
	}
	r := NewRunner(cfg, nil)

	// Use a shell command that writes an error-pattern line to stdout and
	// exits non-zero. The lint parser keys on lines containing the literal
	// word "error" (or "warning"). A bare `false` produces no output, so
	// the parser would mark it as a pass. Forcing the keyword into stdout
	// is what trips the parser to a fail.
	result := r.RunCommand(context.Background(), Command{
		Type:    LintCheck,
		Command: "sh",
		Args:    []string{"-c", "echo 'lint error in file'; exit 1"},
		Timeout: 2 * time.Second,
	})

	if result.Passed {
		t.Error("RunCommand with sh error script: expected Passed = false")
	}
	if result.Error == "" {
		t.Error("RunCommand: expected non-empty Error field")
	}
}

func TestRunner_RunCommand_AddsToHistory(t *testing.T) {
	r := NewRunner(RunnerConfig{DefaultTimeout: 5 * time.Second}, nil)
	r.RunCommand(context.Background(), Command{
		Type:    TypescriptCheck,
		Command: "true",
		Timeout: 2 * time.Second,
	})
	if got := len(r.GetHistory()); got != 1 {
		t.Errorf("after one run, len(history) = %d, want 1", got)
	}
}

func TestRunner_RunCommand_DefaultTimeout(t *testing.T) {
	cfg := RunnerConfig{DefaultTimeout: 5 * time.Second}
	r := NewRunner(cfg, nil)

	// Command with zero timeout should use default
	result := r.RunCommand(context.Background(), Command{
		Type:    TypescriptCheck,
		Command: "true",
		Timeout: 0, // should fall back to cfg.DefaultTimeout
	})
	if !result.Passed {
		t.Errorf("RunCommand with default timeout: %v", result.Error)
	}
}

func TestRunner_RunCommand_RespectsWorkingDir(t *testing.T) {
	// Make a temp dir with a known file
	dir := t.TempDir()
	cfg := RunnerConfig{DefaultTimeout: 5 * time.Second, WorkingDir: dir}
	r := NewRunner(cfg, nil)

	result := r.RunCommand(context.Background(), Command{
		Type:    TypescriptCheck,
		Command: "pwd",
		Timeout: 2 * time.Second,
	})
	if !result.Passed {
		t.Fatalf("pwd failed: %v", result.Error)
	}
	// On macOS, pwd may resolve to /private/var/...; check it's the same dir logically
	// by checking the resolved absolute path of the temp dir
	// The output may include a symlink-resolved path, so we just check the dir name appears
	dirName := filepath.Base(dir)
	if !contains(result.Output, dirName) {
		t.Errorf("pwd output = %q, expected to contain %q", result.Output, dirName)
	}
}

// Helper: small test utility
func contains(haystack, needle string) bool {
	return len(haystack) >= len(needle) && (haystack == needle || indexOf(haystack, needle) >= 0)
}

func indexOf(s, sub string) int {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return i
		}
	}
	return -1
}

func TestRunner_RunAll_RunsAllCommands(t *testing.T) {
	cfg := &config.Config{
		DefaultTimeout: "5s",
		MaxParallel:    2,
		Commands: map[string]config.Command{
			"typescript": {Command: "true", Args: []string{}, Timeout: "5s", Enabled: true},
			"lint":       {Command: "true", Args: []string{}, Timeout: "5s", Enabled: true},
			"test":       {Command: "true", Args: []string{}, Timeout: "5s", Enabled: true},
		},
	}
	r := NewRunner(RunnerConfig{DefaultTimeout: 5 * time.Second}, cfg)
	results := r.RunAll(context.Background())
	if len(results) != 3 {
		t.Errorf("RunAll() returned %d results, want 3", len(results))
	}
}

// DefaultKwatchConfig is a test helper that returns a default kwatch config.
func DefaultKwatchConfig() *config.Config {
	return config.DefaultConfig()
}
