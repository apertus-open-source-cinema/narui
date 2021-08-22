<h1 align="center"><code>narui</code></h1>

A react-inspired UI library for building multimedia desktop apps with rust and vulkan.

* Ergonomics similar to React with hooks
* JSX-like syntax for composing widgets
* clean, readable & familiar looking application code

![narui node graph demo gif](./node_graph_demo.gif)

## Usage
Here is a small introduction of the basic concepts of `narui`. Many things might sound familiar if you used modern react or flutter. 

Make sure to also check out the [examples/](examples/) that cover some more advanced topics and contain more complicated code.

### Basics

`narui` UIs are composed of `widgets`. These building blocks can be anything from a simple box to a complex node graph node or even a whole application. `widgets` are functions that are annotated with the `widget` attribute macro and return either `Fragment` for composed widgets or `FragmentInner` for primitive widgets.
```rust
#[widget]
pub fn square(context: Context) -> Fragment {
    rsx! {
        <rect 
            fill_color=Some(color!(#ffffff)) 
            style={STYLE.width(Points(20.)).height(Points(20.))} 
        />
    }
}
```

The widgets that are defined that way can then be used in other widgets or as the application toplevel via the rsx macro:
```rust
fn main() {
    render(
        WindowBuilder::new(),
        rsx_toplevel! {
            <square />
        },
    );
}

```


### Composition

`narui` follows the principle of composition over inheritance: You build small reusable pieces that then form larger widgets and applications. To enable that, `narui` widgets can have parameters and children.

```rust
#[widget(color = color!(#00aaaa))]  // we assign a default value to the color attribute which is used when color is unspecified
pub fn colored_container(children: Vec<Fragment>, color: Color context: Context) -> Fragment {
    rsx! {
        <rect 
            fill_color=Some(color) 
            style={STYLE.padding(Points(20.))} 
        >
            {children}
        </rect>
    }
}
```

We can then use that component like this:
```rust
rsx! {
        <colored_container>
            <text>{"Hello, world"}</text>
            <square />
        </rect>
    }
```



### State, Hooks & Input handling

State management in `narui` is done using `hooks`. This is also where the ominous `context` struct that is passed to every widget comes into play. Hooks work similiar to react hooks. The most simple hook is the `context.listenable` hook, which is used to store state. Widgets can subscribe to `Listenable`s with the `context.listen` method and get reevaluated when the state that they listen changed. Similiarily, the value of a listenable can be updated by using the `context.shout` method. Listenables should not be updated during the evaluation of a widget but only in reaction to external events. This way, re-render loops can be avoided in a clean and easy way.

```rust
#[widget(initial_value = 1)]
pub fn counter(initial_value: i32, context: Context) -> Fragment {
    let count = context.listenable(initial_value);
    let on_click = move |context: Context| {
        context.shout(count, context.listen(count) + 1)
    };

    rsx! {
        <button on_click=on_click>
            <text>
                {format!("{}", context.listen(count))}
            </text>
        </button>
    }
}
```


### Business logic interaction & interfacing the rest of the world

Interaction with non UI-related code should be done similiar to how interaction with UI related code is done: 
* `Listenables` should signal the state from business logic to the UI.
* Events should signal input from the UI to the business logic. This can be simple callbacks as you would to in your UI code, but it can also be more complicated with `mpsc`s or comparable techniques.

The first step of interacting with Business logic is to run it. This can be done with the `effect` hook manually or by using the `thread` hook as a utility over that. For a simple example of how that can be acomplished, see [examples/stopwatch.rs](examples/stopwatch.rs).


### Custom rendering

`narui` allows `widget`s defined in downstream application code to emit fully custom vulkan api calls including drawcalls. This is especially important for multimedia applications. Example widgets that could be implemented this way are 3D viewports, image / video views and similiar things.
