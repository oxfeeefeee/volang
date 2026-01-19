export interface Example {
  name: string;
  code: string;
}

export const examples: Example[] = [
  {
    name: 'Hello World',
    code: `package main

func main() {
    println("Hello, Vo!")
}
`,
  },
  {
    name: 'Variables',
    code: `package main

func main() {
    // Type inference
    x := 42
    name := "Vo"
    pi := 3.14159
    
    println("x =", x)
    println("name =", name)
    println("pi =", pi)
    
    // Explicit types
    var count int = 100
    var active bool = true
    
    println("count =", count)
    println("active =", active)
}
`,
  },
  {
    name: 'Functions',
    code: `package main

func add(a, b int) int {
    return a + b
}

func swap(a, b int) (int, int) {
    return b, a
}

func main() {
    sum := add(3, 5)
    println("3 + 5 =", sum)
    
    x, y := swap(10, 20)
    println("swapped:", x, y)
}
`,
  },
  {
    name: 'Loops',
    code: `package main

func main() {
    // Classic for loop
    for i := 0; i < 5; i++ {
        println("i =", i)
    }
    
    // While-style loop
    n := 1
    for n < 100 {
        n *= 2
    }
    println("n =", n)
    
    // Range over slice
    nums := []int{10, 20, 30}
    for i, v := range nums {
        println("nums[", i, "] =", v)
    }
}
`,
  },
  {
    name: 'Structs',
    code: `package main

type Point struct {
    X, Y int
}

func (p Point) String() string {
    return "Point"
}

func (p *Point) Move(dx, dy int) {
    p.X += dx
    p.Y += dy
}

func main() {
    p := Point{X: 10, Y: 20}
    println("Initial:", p.X, p.Y)
    
    p.Move(5, -3)
    println("After move:", p.X, p.Y)
}
`,
  },
  {
    name: 'Interfaces',
    code: `package main

type Speaker interface {
    Speak() string
}

type Dog struct {
    Name string
}

func (d Dog) Speak() string {
    return "Woof!"
}

type Cat struct {
    Name string
}

func (c Cat) Speak() string {
    return "Meow!"
}

func greet(s Speaker) {
    println(s.Speak())
}

func main() {
    dog := Dog{Name: "Buddy"}
    cat := Cat{Name: "Whiskers"}
    
    greet(dog)
    greet(cat)
}
`,
  },
  {
    name: 'Closures',
    code: `package main

func counter() func() int {
    count := 0
    return func() int {
        count++
        return count
    }
}

func main() {
    c := counter()
    
    println(c())  // 1
    println(c())  // 2
    println(c())  // 3
    
    c2 := counter()
    println(c2()) // 1 (new counter)
}
`,
  },
  {
    name: 'Fibonacci',
    code: `package main

func fib(n int) int {
    if n <= 1 {
        return n
    }
    return fib(n-1) + fib(n-2)
}

func main() {
    for i := 0; i <= 10; i++ {
        println("fib(", i, ") =", fib(i))
    }
}
`,
  },
];
