use std::collections::HashMap;

use std::io::{self};

use indoc::indoc;

use std::rc::Rc;

use druid::kurbo::{Circle, Line, RoundedRect};
use druid::piet::{FontFamily, Text, TextLayout, TextLayoutBuilder};
use druid::widget::prelude::*;
use druid::widget::{Button, Checkbox, CrossAxisAlignment, Flex, MainAxisAlignment, Widget};
use druid::{
    AppLauncher, Color, Command, Data, Lens, MouseButton, Point, Rect, Selector, Size, Target, WidgetExt, WindowDesc,
};

const PADDING: f64 = 8.0;

// for tree nodes
const RADIUS: f64 = 1.0;
const RECT_RADIUS: f64 = 0.2;
const WIDTH: f64 = 0.05;
const CHILD_SEP: f64 = 0.3;
const LEVEL_SEP: f64 = 1.5;

const BACKGROUND: Color = Color::rgb8(0x30 as u8, 0x30 as u8, 0x30 as u8);

#[derive(Clone, Lens, Data)]
struct AppData {
    g: Rc<Vec<Vec<usize>>>,
    ugly_edges: bool,
    vertex_info: Rc<Vec<String>>,
    only_vertex_info: bool,
    start_from_1: bool,
    edge_info: Rc<Vec<((usize, usize), String)>>,
}

struct DrawingWidget {
    scale: f64,
    center: Point,
    last_mouse_pos: Point,
    size: Size,
    first_time: bool,
    pos: Vec<Point>,
    moving_vertex: bool,
    vertex: usize,
    sizes: Vec<Size>,
}

impl DrawingWidget {
    fn transform(&self, mut p: Point) -> Point {
        p.x = (p.x - self.center.x) * self.scale + self.size.width / 2.0;
        p.y = (p.y - self.center.y) * self.scale + self.size.height / 2.0;
        p
    }

    fn draw_vertex(&self, ctx: &mut PaintCtx, data: &AppData, _env: &Env, mut v: usize, mut pos: Point) {
        pos = self.transform(pos);
        let scaled_radius = RADIUS * self.scale;

        if data.vertex_info.is_empty() {
            if pos.x < -scaled_radius
                || pos.y < -scaled_radius
                || pos.x > self.size.width + scaled_radius
                || pos.y > self.size.height + scaled_radius
            {
                return;
            }

            let circle = Circle::new(pos, scaled_radius);
            ctx.fill(circle, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
            let circle = Circle::new(pos, scaled_radius - WIDTH * self.scale);
            ctx.fill(circle, &BACKGROUND);

            let name = v.to_string();
            let mut font_size = 1.0 * self.scale;
            if name.len() > 3 {
                font_size = font_size / name.len() as f64 * 3.0;
            }

            if data.start_from_1 {
                v += 1;
            }

            let text = ctx.text();
            let layout = text
                .new_text_layout(v.to_string())
                .font(FontFamily::SERIF, font_size)
                .text_color(Color::rgb8(0xff, 0xff, 0xff))
                // .alignment(TextAlignment::Start)
                .build()
                .unwrap();

            let text_size = layout.size();

            let mut text_pos = pos;
            text_pos.x -= text_size.width / 2.0;
            text_pos.y -= text_size.height / 2.0;

            ctx.draw_text(&layout, text_pos);
        } else {
            if pos.x < -self.sizes[v].width / 2.0 * self.scale
                || pos.y < -self.sizes[v].height / 2.0 * self.scale
                || pos.x > self.size.width + self.sizes[v].width / 2.0 * self.scale
                || pos.y > self.size.height + self.sizes[v].height / 2.0 * self.scale
            {
                return;
            }

            let mut num = v;
            if data.start_from_1 {
                num = v + 1;
            }

            let name;
            if data.only_vertex_info {
                name = data.vertex_info[v].clone();
            } else {
                name = num.to_string() + " (" + &data.vertex_info[v] + ")";
            }

            let text = ctx.text();
            let layout = text
                .new_text_layout(name)
                .font(FontFamily::MONOSPACE, 1.0 * self.scale)
                .text_color(Color::rgb8(0xff, 0xff, 0xff))
                // .alignment(TextAlignment::Start)
                .build()
                .unwrap();

            let text_size = layout.size();

            let rect = RoundedRect::from_rect(
                Rect::from_center_size(
                    pos,
                    Size::new(self.sizes[v].width * self.scale, self.sizes[v].height * self.scale),
                ),
                RECT_RADIUS * self.scale,
            );
            ctx.fill(rect, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
            let rect = RoundedRect::from_rect(
                Rect::from_center_size(
                    pos,
                    Size::new(
                        (self.sizes[v].width - WIDTH) * self.scale,
                        (self.sizes[v].height - WIDTH) * self.scale,
                    ),
                ),
                RECT_RADIUS * self.scale,
            );
            ctx.fill(rect, &BACKGROUND);

            let mut text_pos = pos;
            text_pos.x -= text_size.width / 2.0;
            text_pos.y -= text_size.height / 2.0;

            ctx.draw_text(&layout, text_pos);
        }
    }

    fn draw_edge_info(&self, ctx: &mut PaintCtx, _data: &AppData, _env: &Env, u: usize, v: usize, s: &String) {
        let pos = self.transform(Point::new(
            (self.pos[u].x + self.pos[v].x) / 2.0,
            (self.pos[u].y + self.pos[v].y) / 2.0,
        ));

        let text = ctx.text();
        let layout = text
            .new_text_layout(s.clone())
            .font(FontFamily::MONOSPACE, 0.5 * self.scale)
            .text_color(Color::rgb8(0xff, 0xff, 0xff))
            // .alignment(TextAlignment::Start)
            .build()
            .unwrap();

        let mut text_size = layout.size();

        let mut text_pos = pos;
        text_pos.x -= text_size.width / 2.0;
        text_pos.y -= text_size.height / 2.0;

        text_size.width += text_size.height / 2.0;

        let rect = RoundedRect::from_rect(Rect::from_center_size(pos, text_size), RECT_RADIUS * self.scale);
        ctx.fill(rect, &BACKGROUND);
        ctx.stroke(
            rect,
            &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8),
            WIDTH * self.scale / 2.0,
        );

        ctx.draw_text(&layout, text_pos);
    }

    fn init_pos(&mut self, g: &Vec<Vec<usize>>) {
        let mut levels: Vec<Vec<usize>> = vec![Vec::new(); g.len() + 1];
        let mut level: Vec<usize> = vec![0; g.len()];
        let mut used: Vec<bool> = vec![false; g.len()];

        levels[0].push(0);
        used[0] = true;

        for i in 0..levels.len() {
            if levels[i].is_empty() {
                break;
            }
            let mut next: Vec<usize> = Vec::new();
            for &v in levels[i].iter() {
                for &k in g[v].iter() {
                    if !used[k] {
                        used[k] = true;
                        next.push(k);
                        level[k] = i + 1;
                    }
                }
            }
            levels[i + 1] = next;
        }

        let mut child_modifier: Vec<Point> = vec![Point::new(0.0, 0.0); g.len()];
        self.pos = vec![Point::new(0.0, 0.0); g.len()];

        for i in (0..levels.len()).rev() {
            let y = (RADIUS * 2.0 + LEVEL_SEP) * i as f64;
            let mut last_x = 0.0;
            let mut cur_modifier = Point::new(0.0, 0.0);
            for &v in levels[i].iter() {
                let mut first_child = 0.0;
                let mut last_child = 0.0;
                let mut first = true;
                for &k in g[v].iter() {
                    if level[k] == level[v] + 1 {
                        if first {
                            first_child = self.pos[k].x + cur_modifier.x;
                            first = false;
                        }
                        last_child = self.pos[k].x + cur_modifier.x;
                    }
                }

                child_modifier[v].x = cur_modifier.x;

                if first { // leaf
                } else {
                    let mid = (first_child + last_child) / 2.0;
                    if mid < last_x {
                        child_modifier[v].x += last_x - mid;
                        cur_modifier.x += last_x - mid;
                    } else {
                        last_x = mid;
                    }
                }

                self.pos[v] = Point::new(last_x, y);

                last_x += RADIUS * 2.0 + CHILD_SEP;
            }
        }

        for i in 0..levels.len() {
            for &v in levels[i].iter() {
                for &k in g[v].iter() {
                    if level[k] == level[v] + 1 {
                        self.pos[k].x += child_modifier[v].x;
                        self.pos[k].y += child_modifier[v].y;

                        child_modifier[k].x += child_modifier[v].x;
                        child_modifier[k].y += child_modifier[v].y;
                    }
                }
            }
        }
    }

    fn init_sizes(&mut self, ctx: &mut PaintCtx, data: &AppData) {
        let scaled_radius = RADIUS * self.scale;

        self.sizes = vec![Size::new(0.0, 0.0); data.g.len()];

        for v in 0..data.g.len() {
            let name;
            if data.only_vertex_info {
                name = data.vertex_info[v].clone();
            } else {
                name = v.to_string() + " (" + &data.vertex_info[v] + ")";
            }

            let text = ctx.text();
            let layout = text
                .new_text_layout(name)
                .font(FontFamily::MONOSPACE, 1.0 * self.scale)
                .text_color(Color::rgb8(0xff, 0xff, 0xff))
                // .alignment(TextAlignment::Start)
                .build()
                .unwrap();

            let text_size = layout.size();
            self.sizes[v] = Size::new(
                (text_size.width + scaled_radius).max(scaled_radius * 2.0),
                scaled_radius * 2.0,
            );
            self.sizes[v].width /= self.scale;
            self.sizes[v].height /= self.scale;
        }
    }

    fn inside_node(&self, data: &AppData, v: usize, pos: Point) -> bool {
        let vertex = self.transform(self.pos[v]);

        if data.vertex_info.is_empty() {
            return vertex.distance(pos) < RADIUS * self.scale;
        } else {
            let dx = (pos.x - vertex.x).abs() / self.scale;
            let dy = (pos.y - vertex.y).abs() / self.scale;
            return dx * 2.0 < self.sizes[v].width && dy * 2.0 < self.sizes[v].height;
        }
    }

    // returns width of a tree
    fn init_pos_info(&mut self, g: &Vec<Vec<usize>>) -> f64 {
        let mut levels: Vec<Vec<usize>> = vec![Vec::new(); g.len() + 1];
        let mut level: Vec<usize> = vec![0; g.len()];
        let mut used: Vec<bool> = vec![false; g.len()];

        levels[0].push(0);
        used[0] = true;

        for i in 0..levels.len() {
            if levels[i].is_empty() {
                break;
            }
            let mut next: Vec<usize> = Vec::new();
            for &v in levels[i].iter() {
                for &k in g[v].iter() {
                    if !used[k] {
                        used[k] = true;
                        next.push(k);
                        level[k] = i + 1;
                    }
                }
            }
            levels[i + 1] = next;
        }

        let mut child_modifier: Vec<Point> = vec![Point::new(0.0, 0.0); g.len()];
        let mut widths: Vec<f64> = vec![0.0; g.len()];
        self.pos = vec![Point::new(0.0, 0.0); g.len()];

        for i in (0..levels.len()).rev() {
            let y = (RADIUS * 2.0 + LEVEL_SEP) * i as f64;
            for &v in levels[i].iter() {
                let mut first = true;
                for &k in g[v].iter() {
                    if level[k] == level[v] + 1 {
                        if first {
                            first = false;
                        } else {
                            widths[v] += CHILD_SEP;
                        }
                        child_modifier[k].x += widths[v];
                        widths[v] += widths[k];
                    }
                }
                if widths[v] < self.sizes[v].width {
                    let dif = (self.sizes[v].width - widths[v]) / 2.0;
                    for &k in g[v].iter() {
                        if level[k] == level[v] + 1 {
                            child_modifier[k].x += dif;
                        }
                    }
                    widths[v] = self.sizes[v].width;
                }
                self.pos[v] = Point::new(widths[v] / 2.0, y);
            }
        }

        for i in 0..levels.len() {
            for &v in levels[i].iter() {
                self.pos[v].x += child_modifier[v].x;
                self.pos[v].y += child_modifier[v].y;
                for &k in g[v].iter() {
                    if level[k] == level[v] + 1 {
                        child_modifier[k].x += child_modifier[v].x;
                        child_modifier[k].y += child_modifier[v].y;
                    }
                }
            }
        }

        widths[0]
    }

    fn refresh(&mut self, ctx: &mut PaintCtx, data: &AppData) {
        let size: Size = ctx.size();
        if !data.vertex_info.is_empty() {
            self.init_sizes(ctx, &data);
            let width = self.init_pos_info(&data.g);
            self.first_time = false;
            let mut height: f64 = 0.0;
            for &p in self.pos.iter() {
                height = height.max(p.y);
            }
            self.center = Point::new(width / 2.0, height / 2.0);
            height += RADIUS * 2.0;
            self.scale = (size.width / width).min(size.height / height) * 0.8;
        } else {
            self.init_pos(&data.g);
            self.first_time = false;
            let mut width: f64 = 0.0;
            let mut height: f64 = 0.0;
            for &p in self.pos.iter() {
                width = width.max(p.x);
                height = height.max(p.y);
            }
            self.center = Point::new(width / 2.0, height / 2.0);
            width += RADIUS * 2.0;
            height += RADIUS * 2.0;
            self.scale = (size.width / width).min(size.height / height) * 0.8;
        }
    }
}

impl Widget<AppData> for DrawingWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppData, _env: &Env) {
        match event {
            Event::MouseMove(e) => {
                if e.buttons.contains(MouseButton::Left) {
                    if self.moving_vertex {
                        self.pos[self.vertex].x += (e.pos.x - self.last_mouse_pos.x) / self.scale;
                        self.pos[self.vertex].y += (e.pos.y - self.last_mouse_pos.y) / self.scale;
                        self.last_mouse_pos = e.pos;
                    } else {
                        self.center.x -= (e.pos.x - self.last_mouse_pos.x) / self.scale;
                        self.center.y -= (e.pos.y - self.last_mouse_pos.y) / self.scale;
                        self.last_mouse_pos = e.pos;
                    }
                    ctx.request_paint();
                }
            }
            Event::Wheel(e) => {
                self.scale = self.scale * 0.01_f64.max(1.1_f64.powf(-e.wheel_delta.y / 25.0));
                ctx.request_paint();
            }
            Event::MouseDown(e) => {
                self.last_mouse_pos = e.pos.clone();
                self.moving_vertex = false;
                for i in 0..data.g.len() {
                    if self.inside_node(data, i, e.pos) {
                        self.moving_vertex = true;
                        self.vertex = i;
                    }
                }
            }
            Event::Command(_) => {
                self.first_time = true;
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &AppData, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(&mut self, _layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &AppData, _env: &Env) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, env: &Env) {
        let size: Size = ctx.size();
        self.size = size.clone();

        ctx.fill(Rect::from_origin_size(Point { x: 0.0, y: 0.0 }, size), &BACKGROUND);

        if self.first_time {
            self.refresh(ctx, data);
        }

        let mut delta: f64 = 0.0;
        if data.ugly_edges {
            delta = RADIUS;
        }

        for i in 0..data.g.len() {
            for &j in data.g[i].iter() {
                if self.pos[i].y < self.pos[j].y {
                    ctx.stroke(
                        Line::new(
                            self.transform(Point::new(self.pos[i].x, self.pos[i].y + delta)),
                            self.transform(Point::new(self.pos[j].x, self.pos[j].y - delta)),
                        ),
                        &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8),
                        WIDTH * self.scale,
                    );
                }
            }
        }

        for ((u, v), s) in data.edge_info.iter() {
            self.draw_edge_info(ctx, data, env, *u, *v, s);
        }

        for i in 0..data.g.len() {
            self.draw_vertex(ctx, data, env, i, self.pos[i]);
        }
    }
}

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw tree [flags]

            Draws a tree

            Flags:
                --help                 Display this message
                --ugly-edges           Draw edges in a different way to make sure they don't
                                       intersect nodes.
                --vertex-info, -vi     Add some information to the nodes.
                  -vi=[opt1],[opt2]    Configure options for vertex info
                  -vi=only             Don't show vertex indices, only info
                  -vi=lines            Read info for each vertex from new line (by default
                                       it reads one line and splits it by spaces)
                --edge-info, -ei       Add edge information (on the same line with the
                                       corresponding edge)
        "};
        print!("{}", s);
        return;
    }

    let mut app_data = AppData {
        g: Rc::new(Vec::new()),
        ugly_edges: false,
        vertex_info: Rc::new(Vec::new()),
        only_vertex_info: false,
        start_from_1: false,
        edge_info: Rc::new(Vec::new()),
    };

    let mut is_vertex_info = false;
    let mut vertex_info_lines = false;
    let mut is_edge_info = false;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "--ugly-edges" {
            app_data.ugly_edges = true;
        } else if args[i].starts_with("--vertex-info") || args[i].starts_with("-vi") {
            is_vertex_info = true;
            if args[i].contains("only") {
                app_data.only_vertex_info = true;
            }
            if args[i].contains("lines") {
                vertex_info_lines = true;
            }
        } else if args[i] == "--edge-info" || args[i] == "-ei" {
            is_edge_info = true;
        } else {
            eprintln!("Unknown option \"{}\"", args[i]);
            std::process::exit(1);
        }
        i += 1;
    }

    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    let n: usize = s.trim().split(" ").next().unwrap().parse().unwrap();

    if is_vertex_info {
        let mut vertex_info: Vec<String> = Vec::new();
        if !vertex_info_lines {
            let mut s = String::new();
            io::stdin().read_line(&mut s).unwrap();
            vertex_info = s.trim().split(" ").map(|x| x.to_string()).collect::<Vec<_>>();
        } else {
            for _i in 0..n {
                let mut s = String::new();
                io::stdin().read_line(&mut s).unwrap();
                vertex_info.push(s.trim().to_string());
            }
        }
        if vertex_info.len() < n {
            eprintln!("Not enough vertex info: need {}, got {}", n, vertex_info.len());
            std::process::exit(1);
        }
        app_data.vertex_info = Rc::new(vertex_info);
    }

    let mut g: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut edge_info: Vec<((usize, usize), String)> = Vec::new();
    for _i in 0..n - 1 {
        s.clear();
        io::stdin().read_line(&mut s).unwrap();
        let mut iter = s.trim().split(" ");
        let mut u = iter.next().unwrap().parse::<usize>().unwrap();
        let mut v = iter.next().unwrap().parse::<usize>().unwrap();
        u -= 1;
        v -= 1;
        if is_edge_info {
            edge_info.push(((u, v), iter.collect::<Vec<_>>().join(" ")));
        }
        g[u].push(v);
        g[v].push(u);
    }
    if is_edge_info {
        app_data.edge_info = Rc::new(edge_info);
    }

    app_data.g = Rc::new(g);

    let window = WindowDesc::new(make_layout())
        .window_size(Size {
            width: 800.0,
            height: 600.0,
        })
        .resizable(true)
        .title("Drawing");
    AppLauncher::with_window(window)
        .launch(app_data)
        .expect("launch failed");
}

fn make_layout() -> impl Widget<AppData> {
    let drawing_widget_id = WidgetId::next();

    Flex::row()
        .with_flex_child(
            DrawingWidget {
                scale: 100.0,
                center: Point::new(0.0, 0.0),
                last_mouse_pos: Point::new(0.0, 0.0),
                size: Size::new(0.0, 0.0),
                first_time: true,
                pos: Vec::new(),
                moving_vertex: false,
                vertex: 0,
                sizes: Vec::new(),
            }
            .with_id(drawing_widget_id),
            1.0,
        )
        .with_spacer(PADDING)
        .with_child(
            Flex::column()
                .with_child(Checkbox::new("Start from 1").lens(AppData::start_from_1))
                .with_spacer(PADDING)
                .with_child(Checkbox::new("Ugly edges").lens(AppData::ugly_edges))
                .with_spacer(PADDING)
                .with_child(Button::new("Refresh").on_click(move |ctx: &mut EventCtx, _data, _env| {
                    ctx.submit_command(Command::new(
                        Selector::new("refresh"),
                        (),
                        Target::Widget(drawing_widget_id),
                    ));
                }))
                .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::End)
        .padding(PADDING)
}
