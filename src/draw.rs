use indoc::indoc;

use std::collections::{HashMap};

use std::io::{self};

use std::rc::Rc;

use druid::widget::prelude::*;
use druid::widget::{Flex, Button, Widget, MainAxisAlignment, CrossAxisAlignment, Checkbox};
use druid::{Size, AppLauncher, WindowDesc, Data, Lens, Color, Rect, Point, WidgetExt};
use druid::kurbo::{Circle, Line};

const PADDING: f64 = 8.0;

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if args.is_empty() || args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw [type] [flags]

            Draws. Use \"cpr draw [type] --help\" for more info.

            Types:
                pts, points         Draw points on a plane

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }
    if args[0] == "pts" || args[0] == "points" {
        draw_points(&args[1..].to_vec(), _params);
    }
}

#[derive(Clone, Lens, Data)]
struct AppData {
    points: Rc<Vec<(f64, f64)>>,
    mouse: Point,
    show_plus_target: bool,
    show_x_target: bool,
}

struct DrawingWidget {
}

impl Widget<AppData> for DrawingWidget {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut AppData, _env: &Env) {
        data.mouse = Point{x: -1.0, y: -1.0};
        match event {
            Event::MouseMove(e) => {
                data.mouse = e.pos.clone();
            },
            _ => (),
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppData,
        _env: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppData,
        _env: &Env,
    ) -> Size {
        Size {
            width: bc.max().width,
            height: bc.max().height,
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let size: Size = ctx.size();
        ctx.fill(Rect::from_origin_size(Point{x: 0.0, y: 0.0}, size), &Color::rgb8(0x30 as u8, 0x30 as u8, 0x30 as u8));
        let mut left: f64 = 0.0;
        let mut right: f64 = 0.0;
        let mut up: f64 = 0.0;
        let mut down: f64 = 0.0;
        for (x, y) in data.points.iter().map(|&(x, y)| (x as f64, -y as f64)) {
            left = left.min(x);
            right = right.max(x);
            down = down.min(y);
            up = up.max(y);
        }
        left -= 0.5;
        right += 0.5;
        up += 0.5;
        down -= 0.5;

        let cell_width = size.width / (right - left) as f64;
        let cell_height = size.height / (up - down) as f64;
        let cell_size;
        if cell_width > cell_height {
            cell_size = cell_height;
        } else {
            cell_size = cell_width;
        }
        left *= cell_width / cell_size;
        // right *= cell_width / cell_size;
        // up *= cell_height / cell_size;
        down *= cell_height / cell_size;

        let grid_color = Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8);

        ctx.stroke(Line::new(Point{x: 0.0, y: -down * cell_size}, Point{x: size.width, y: -down * cell_size}), &grid_color, 2.0);
        ctx.stroke(Line::new(Point{x: -left * cell_size, y: 0.0}, Point{x: -left * cell_size, y: size.height}), &grid_color, 2.0);

        let mut y = (down.ceil() - down) * cell_size;
        while y < size.height {
            ctx.stroke(Line::new(Point{x: 0.0, y: y}, Point{x: size.width, y: y}), &grid_color, 0.3);
            y += cell_size;
        }

        let mut x = (left.ceil() - left) * cell_size;
        while x < size.width {
            ctx.stroke(Line::new(Point{x: x, y: 0.0}, Point{x: x, y: size.height}), &grid_color, 0.3);
            x += cell_size;
        }

        for (x, y) in data.points.iter().map(|&(x, y)| (x as f64, -y as f64)) {
            let posx = (x - left) * cell_size;
            let posy = (y - down) * cell_size;
            let point = Circle::new(Point{x: posx, y: posy}, 5.0);
            ctx.fill(point, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
        }

        let target_color = Color::rgb8(0xff as u8, 0 as u8, 0 as u8);
        if data.show_plus_target && data.mouse.x > 0.0 && data.mouse.y > 0.0 && data.mouse.x < size.width && data.mouse.y < size.height {
            ctx.stroke(Line::new(Point{x: 0.0, y: data.mouse.y}, Point{x: size.width, y: data.mouse.y}), &target_color, 1.5);
            ctx.stroke(Line::new(Point{x: data.mouse.x, y: 0.0}, Point{x: data.mouse.x, y: size.height}), &target_color, 1.5);
        }

        if data.show_x_target && data.mouse.x > 0.0 && data.mouse.y > 0.0 && data.mouse.x < size.width && data.mouse.y < size.height {
            let down_left = data.mouse.x.min(data.mouse.y);
            ctx.stroke(Line::new(Point{x: data.mouse.x - down_left, y: data.mouse.y - down_left}, data.mouse), &target_color, 1.0);

            let up_right = (size.width - data.mouse.x).min(size.height - data.mouse.y);
            ctx.stroke(Line::new(Point{x: data.mouse.x + up_right, y: data.mouse.y + up_right}, data.mouse), &target_color, 1.0);

            let up_left = data.mouse.x.min(size.height - data.mouse.y);
            ctx.stroke(Line::new(Point{x: data.mouse.x - up_left, y: data.mouse.y + up_left}, data.mouse), &target_color, 1.0);

            let down_right = (size.width - data.mouse.x).min(data.mouse.y);
            ctx.stroke(Line::new(Point{x: data.mouse.x + down_right, y: data.mouse.y - down_right}, data.mouse), &target_color, 1.0);
        }
    }
}

fn draw_points(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw points [flags]

            Draws points

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }

    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    let n: i32 = s.trim().split(" ").next().unwrap().parse().unwrap();
    for _i in 0..n {
        s.clear();
        io::stdin().read_line(&mut s).unwrap();
        let mut iter = s.trim().split(" ").map(|x| x.parse::<f64>().unwrap());
        points.push((iter.next().unwrap(), iter.next().unwrap()));
    }

    let window = WindowDesc::new(make_layout)
        .window_size(Size {
            width: 800.0,
            height: 600.0,
        })
        .resizable(true)
        .title("Drawing");
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(AppData{
            points: Rc::new(points),
            mouse: Point{x: 0.0, y: 0.0},
            show_plus_target: true,
            show_x_target: true,
        })
        .expect("launch failed");
}

fn make_layout() -> impl Widget<AppData> {
    Flex::row()
        .with_flex_child(
            DrawingWidget{},
            1.0
        )
        .with_spacer(PADDING)
        .with_child(
            Flex::column()
                .with_child(Checkbox::new("Show + target").lens(AppData::show_plus_target))
                .with_spacer(PADDING)
                .with_child(Checkbox::new("Show x target").lens(AppData::show_x_target))
                .cross_axis_alignment(CrossAxisAlignment::Start)
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::End)
        .padding(PADDING)
}
