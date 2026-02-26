package main

import (
	"encoding/json"
	"fmt"
	"time"
)

type Event struct {
	Name      string            `json:"name"`
	Timestamp time.Time         `json:"timestamp"`
	Meta      map[string]string `json:"meta"`
}

func main() {
	e := Event{
		Name:      "demo.highlight",
		Timestamp: time.Now().UTC(),
		Meta: map[string]string{
			"lang": "go",
			"env":  "demo",
		},
	}

	payload, err := json.MarshalIndent(e, "", "  ")
	if err != nil {
		panic(err)
	}

	fmt.Printf("event=%s\n", payload)
}
