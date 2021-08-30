use crate::{layouter::Layouter, layouts::*, *};

#[test]
fn smoke_test() {
    let mut layouter = Layouter::<Box<dyn Layout>>::new();
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

    let column = layouter.add_node(column);
    let box_a = layouter.add_node(box_a);
    let box_b = layouter.add_node(box_b);
    let box_c = layouter.add_node(box_c);
    let box_d = layouter.add_node(box_d);
    let flex = layouter.add_node(flexible);

    layouter.set_children(flex, [box_b].iter().cloned());
    layouter.set_children(column, [box_a, flex, box_c, box_d].iter().cloned());

    layouter.do_layout(
        BoxConstraints::tight_for(Size { width: 100.0, height: 100.0 }),
        Offset::zero(),
        column,
    );

    macro_rules! assert_layout {
        ($result:expr, $golden:expr) => {
            let result = $result;
            assert_eq!(result.0, $golden.0);
            assert_eq!(result.1, $golden.1);
        };
    }

    assert_layout!(
        layouter.get_layout(box_a),
        (Offset { x: 30.0, y: 0.0 }, Size { width: 40.0, height: 20.0 })
    );
    assert_layout!(
        layouter.get_layout(box_b),
        (Offset { x: 45.0, y: 20.0 }, Size { width: 10.0, height: 10.0 })
    );
    assert_layout!(
        layouter.get_layout(box_c),
        (Offset { x: 40.0, y: 30.0 }, Size { width: 20.0, height: 10.0 })
    );
    assert_layout!(
        layouter.get_layout(box_d),
        (Offset { x: 32.5, y: 40.0 }, Size { width: 35.0, height: 25.0 })
    );

    let flexible = Box::new(Flexible { flex: Flex { flex: 1.0, fit: FlexFit::Tight } });

    layouter.set_node(flex, flexible);
    layouter.do_layout(
        BoxConstraints::tight_for(Size { width: 100.0, height: 100.0 }),
        Offset::zero(),
        column,
    );

    assert_layout!(
        layouter.get_layout(box_a),
        (Offset { x: 30.0, y: 0.0 }, Size { width: 40.0, height: 20.0 })
    );
    assert_layout!(
        layouter.get_layout(box_b),
        (Offset { x: 45.0, y: 20.0 }, Size { width: 10.0, height: 45.0 })
    );
    assert_layout!(
        layouter.get_layout(box_c),
        (Offset { x: 40.0, y: 65.0 }, Size { width: 20.0, height: 10.0 })
    );
    assert_layout!(
        layouter.get_layout(box_d),
        (Offset { x: 32.5, y: 75.0 }, Size { width: 35.0, height: 25.0 })
    );
}
