package main

import (
	"fmt"
	"os"
)

func sum(values []int) int {
	total := 0
	for _, v := range values {
		total += v
	}
	return total
}

func main() {
	fmt.Println(sum([]int{1, 2, 3}))
	if len(os.Args) > 1 {
		fmt.Println(os.Args[1])
	}
}
