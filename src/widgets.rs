#[widget]
fn row(children: Vec<TreeNode>) -> TreeNode {
    TreeNode {
        style: Style::default(),
        children: TreeChildren::Children(children)
    }
}

#[widget]
fn button(text: String) -> TreeNode {
    rsx! {
        <stacked>
            <rounded_rect/>
            <text>{text}</text>
        </stacked>
    }
}

#[widget]
fn h_splitter(a: TreeNode, b: TreeNode) -> TreeNode {
    let percent_split = use_state!(50.0);

    rsx! {
        <row>
            <sized_box width_percent={percent_split}>{a}</sized_box>
            {b}
        </row>
    };
}

#[widget]
fn numeric_input(value: f64) -> TreeNode {
    rsx! {
        <stacked>
            <rounded_rect/>
            <text>{format!("{:d}", value)}</text>
        </stacked>
    }
}