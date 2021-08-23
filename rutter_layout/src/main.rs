use layout::{
    BoxConstraints,
    Column,
    CrossAxisAlignment,
    Flex,
    FlexFit,
    Flexible,
    Layoutable,
    Layouter,
    MainAxisAlignment,
    MainAxisSize,
    Offset,
    Size,
    SizedBox,
};

// Interesting widgets:
// Align
// Positioned
// Padding
// FractionallySizedBox

fn main() {
    let mut layouter = Layouter::<&str, Box<dyn Layoutable>>::new();
    let column = Box::new(Column {
        cross_axis_alignment: CrossAxisAlignment::Center,
        main_axis_alignment: MainAxisAlignment::Start,
        main_axis_size: MainAxisSize::Max,
    });

    let box_a = Box::new(SizedBox::new(Size { width: 40.0, height: 20.0 }));
    let flexible = Box::new(Flexible { flex: Flex { flex: 1.0, fit: FlexFit::Loose } });
    let box_b = Box::new(SizedBox::constrained(BoxConstraints {
        min_width: 10.0,
        max_width: 40.0,
        min_height: 10.0,
        max_height: 25.0,
    }));
    let box_c = Box::new(SizedBox::new(Size { width: 20.0, height: 10.0 }));
    let box_d = Box::new(SizedBox::new(Size { width: 35.0, height: 25.0 }));

    layouter.set_node("column", column);
    layouter.set_node("a", box_a);
    layouter.set_node("b", box_b);
    layouter.set_node("c", box_c);
    layouter.set_node("d", box_d);
    layouter.set_node("flex", flexible);

    layouter.set_children("flex", &["b"]);
    layouter.set_children("column", &["a", "flex", "c", "d"]);

    layouter.do_layout(
        BoxConstraints::tight_for(Size { width: 100.0, height: 100.0 }),
        Offset::zero(),
        "column",
    );

    println!("{:?}", layouter.get_layout("a"));
    println!("{:?}", layouter.get_layout("b"));
    println!("{:?}", layouter.get_layout("c"));
    println!("{:?}", layouter.get_layout("d"));

    let _box_b = Box::new(SizedBox::constrained(BoxConstraints {
        min_width: 10.0,
        max_width: 40.0,
        min_height: 35.0,
        max_height: 55.0,
    }));
    let flexible = Box::new(Flexible { flex: Flex { flex: 1.0, fit: FlexFit::Tight } });

    layouter.set_node("flex", flexible);
    layouter.do_layout(
        BoxConstraints::tight_for(Size { width: 100.0, height: 100.0 }),
        Offset::zero(),
        "column",
    );

    println!();
    println!("{:?}", layouter.get_layout("a"));
    println!("{:?}", layouter.get_layout("b"));
    println!("{:?}", layouter.get_layout("c"));
    println!("{:?}", layouter.get_layout("d"));
}
