use crate::layout::{positioned, sized, stack};
use narui::{layout::Maximal, *};
use narui_macros::{rsx, widget};


#[widget(border_radius = Dimension::default(), fill = None, stroke = None)]
pub fn rect_leaf(
    border_radius: Dimension,
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Leaf {
        render_object: RenderObject::RoundedRect {
            stroke_color: stroke.map(|v| v.0),
            fill_color: fill,
            stroke_width: stroke.map(|v| v.1).unwrap_or(0.0),
            border_radius,
            inverted: false,
        },
        layout: Box::new(Maximal),
    }
}


#[widget(border_radius = Dimension::default(), fill = None)]
pub fn inverse_rect_leaf(
    border_radius: Dimension,
    fill: Option<Color>,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Leaf {
        render_object: RenderObject::RoundedRect {
            inverted: true,
            stroke_color: None,
            fill_color: fill,
            stroke_width: 0.0,
            border_radius,
        },
        layout: Box::new(Maximal),
    }
}

#[widget(
    border_radius = Default::default(),
    fill = None,
    stroke = None,
    do_clipping = false,
    constraint = Default::default(),
    children = None,
)]
pub fn rect(
    border_radius: Dimension,
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
    do_clipping: bool,
    constraint: BoxConstraints,
    children: Option<Fragment>,
    context: &mut WidgetContext,
) -> Fragment {
    if !do_clipping {
        if let Some(frag) = children {
            rsx! {
                <stack>
                    <positioned>
                        <rect_leaf border_radius=border_radius fill=fill stroke=stroke />
                    </positioned>
                    <sized constraint=constraint>{frag.into()}</sized>
                </stack>
            }
        } else {
            rsx! {
                <sized constraint=constraint><rect_leaf border_radius=border_radius fill=fill stroke=stroke /></sized>
            }
        }
    } else {
        rsx! {
            <stack is_clipper=true>
                <positioned>
                    <rect_leaf border_radius=border_radius fill=fill stroke=stroke />
                </positioned>
                <sized constraint=constraint>{children}</sized>
                <positioned>
                    <inverse_rect_leaf fill=Some(Color::new(0., 0., 0., 0.)) border_radius=border_radius />
                </positioned>
            </stack>
        }
    }
}
