use std::collections::{HashMap};

use std::io::{self};

use indoc::indoc;

use std::rc::Rc;

use druid::widget::prelude::*;
use druid::widget::{Flex, Widget, MainAxisAlignment, CrossAxisAlignment, Checkbox, Button};
use druid::{
    Size, AppLauncher, WindowDesc, Data, Lens, Color, Rect, Point, WidgetExt, MouseButton,
    Command, Target, Selector
};
use druid::kurbo::{Circle, Line, RoundedRect, BezPath, Vec2};
use druid::piet::{FontFamily, Text, TextLayoutBuilder, TextLayout};

const PADDING: f64 = 8.0;

// for graph nodes
const RADIUS: f64 = 1.0;
const RECT_RADIUS: f64 = 0.2;
const WIDTH: f64 = 0.05;

// for edges
const ARC_OFFSET: f64 = 1.5;
const ARROW_LEN: f64 = 0.6;
const ARROW_ANGLE: f64 = 0.35;

const BACKGROUND: Color = Color::rgb8(0x30 as u8, 0x30 as u8, 0x30 as u8);

#[derive(Clone, Lens, Data)]
struct AppData {
    g: Rc<Vec<Vec<usize>>>,
    vertex_info: Rc<Vec<String>>,
    only_vertex_info: bool,
    start_from_1: bool,
    draw_arcs: bool,
    edge_info: Rc<Vec<((usize, usize), String)>>,
    is_edge_info: bool,
    directed: bool,
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
            if pos.x < -scaled_radius || pos.y < -scaled_radius || pos.x > self.size.width + scaled_radius || pos.y > self.size.height + scaled_radius {
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
            if pos.x < -self.sizes[v].width / 2.0 * self.scale                   || pos.y < -self.sizes[v].height / 2.0 * self.scale ||
               pos.x >  self.size.width + self.sizes[v].width / 2.0 * self.scale || pos.y >  self.size.height + self.sizes[v].height / 2.0 * self.scale {
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

            let rect = RoundedRect::from_rect(Rect::from_center_size(pos,
                        Size::new(self.sizes[v].width * self.scale, self.sizes[v].height * self.scale)),
                        RECT_RADIUS * self.scale);
            ctx.fill(rect, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
            let rect = RoundedRect::from_rect(Rect::from_center_size(pos,
                        Size::new((self.sizes[v].width - WIDTH) * self.scale, (self.sizes[v].height - WIDTH) * self.scale)),
                        RECT_RADIUS * self.scale);
            ctx.fill(rect, &BACKGROUND);

            let mut text_pos = pos;
            text_pos.x -= text_size.width / 2.0;
            text_pos.y -= text_size.height / 2.0;

            ctx.draw_text(&layout, text_pos);
        }
    }

    fn draw_edge_info(&self, ctx: &mut PaintCtx, _data: &AppData, _env: &Env, pos: Point, s: &String) {
        let text = ctx.text();
        let layout = text
            .new_text_layout(s.clone())
            .font(FontFamily::MONOSPACE, 0.65 * self.scale)
            .text_color(Color::rgb8(0xff, 0xff, 0xff))
            // .alignment(TextAlignment::Start)
            .build()
            .unwrap();

        let mut text_size = layout.size();

        let mut text_pos = pos;
        text_pos.x -= text_size.width / 2.0;
        text_pos.y -= text_size.height / 2.0;

        text_size.width += text_size.height / 2.0;

        let rect = RoundedRect::from_rect(Rect::from_center_size(pos,
                    text_size),
                    RECT_RADIUS * self.scale);
        ctx.fill(rect, &BACKGROUND);
        ctx.stroke(rect, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8), WIDTH * self.scale / 2.0);


        ctx.draw_text(&layout, text_pos);
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

    fn init_pos(&mut self, g: &Vec<Vec<usize>>) -> f64 {
        self.pos = vec![Point::new(0.0, 0.0); g.len()];

        let n = g.len();

        let rad = RADIUS * n as f64 * 2.;

        for i in 0..n {
            let angle = std::f64::consts::PI * 2. * (1_f64 / n as f64 * i as f64 + 0.5);
            self.pos[i] = Point::new(angle.cos() * rad, angle.sin() * rad);
        }
        return rad * 2. + RADIUS * 2.;
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
            self.sizes[v] = Size::new((text_size.width + scaled_radius).max(scaled_radius * 2.0), scaled_radius * 2.0);
            self.sizes[v].width /= self.scale;
            self.sizes[v].height /= self.scale;
        }
    }


    fn refresh(&mut self, ctx: &mut PaintCtx, data: &AppData) {
        let size: Size = ctx.size();

        if !data.vertex_info.is_empty() {
            self.init_sizes(ctx, &data);
        }

        self.first_time = false;
        let width = self.init_pos(&data.g);
        let height = width;
        self.center = Point::new(0.0, 0.0);
        self.scale = (size.width / width).min(size.height / height) * 0.8;
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
            },
            Event::Wheel(e) => {
                self.scale = self.scale * 0.01_f64.max(1.1_f64.powf(-e.wheel_delta.y / 25.0));
                ctx.request_paint();
            },
            Event::MouseDown(e) => {
                self.last_mouse_pos = e.pos.clone();
                self.moving_vertex = false;
                for i in 0..data.g.len() {
                    if self.inside_node(data, i, e.pos) {
                        self.moving_vertex = true;
                        self.vertex = i;
                    }
                }
            },
            Event::Command(_) => {
                self.first_time = true;
                ctx.request_paint();
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
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, env: &Env) {
        let size: Size = ctx.size();
        self.size = size.clone();

        ctx.fill(Rect::from_origin_size(Point{x: 0.0, y: 0.0}, size), &BACKGROUND);

        if self.first_time {
            self.refresh(ctx, data);
        }
        for ((ii, jj), s) in data.edge_info.iter() {
            let i = *ii;
            let j = *jj;

            let label_pos: Point;
            let arrow_pos: Point;

            if data.draw_arcs {
                let dist = self.pos[i].distance(self.pos[j]);
                let a = ARC_OFFSET.min(dist * 0.4);
                let h = (dist * dist / 4. - a * a) / 2. / a;
                let mid = self.pos[i].midpoint(self.pos[j]);

                let mut ort = Point::new(-(self.pos[j].y - self.pos[i].y), self.pos[j].x - self.pos[i].x);
                let lnd = ort.distance(Point::new(0., 0.));
                ort.x /= lnd;
                ort.y /= lnd;

                label_pos = Point::new(mid.x + ort.x * -a, mid.y + ort.y * -a);

                let h2 = (h + a) * (h + a) / h - h;
                ort.x *= -h2;
                ort.y *= -h2;
                let center2 = Point::new(ort.x + mid.x, ort.y + mid.y);

                let mut v1 = Point::new(center2.x - self.pos[i].x, center2.y - self.pos[i].y);
                let ln1 = v1.distance(Point::new(0., 0.));
                v1.x /= ln1; v1.y /= ln1;
                let mut ab = Point::new(self.pos[i].x - self.pos[j].x, self.pos[i].y - self.pos[j].y);
                let lab = ab.distance(Point::new(0., 0.));
                ab.x /= lab; ab.y /= lab;
                let lnx = (ab.x * v1.x + ab.y * v1.y).abs();
                let lny = (1_f64 - lnx * lnx).sqrt();
                let needln = a * 4. / 3. / lny;
                v1.x *= needln; v1.y *= needln;

                let mut v2 = Point::new(center2.x - self.pos[j].x, center2.y - self.pos[j].y);
                let ln2 = v2.distance(Point::new(0., 0.));
                v2.x /= ln2; v2.y /= ln2;
                v2.x *= needln; v2.y *= needln;

                v1 = Point::new(v1.x + self.pos[i].x, v1.y + self.pos[i].y);
                v2 = Point::new(v2.x + self.pos[j].x, v2.y + self.pos[j].y);

                let mut path = BezPath::new();
                path.move_to(self.transform(self.pos[i]));
                path.curve_to(self.transform(v1), self.transform(v2), self.transform(self.pos[j]));
                ctx.stroke(path,
                    &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8), WIDTH * self.scale);

                let p = vec![self.pos[i].to_vec2(), v1.to_vec2(), v2.to_vec2(), self.pos[j].to_vec2()];

                let getp = |t: f64| {
                    p[0] * (1. - t).powi(3) + p[1] * (1. - t).powi(2) * t * 3. + p[2] * 3. * t.powi(2) * (1. - t) + p[3] * t.powi(3)
                };

                {
                    let mut l = 0_f64;
                    let mut r = 1_f64;
                    for _it in 0..50 {
                        let c = (l + r) / 2.;
                        if self.inside_node(&data, j, self.transform(getp(c).to_point())) {
                            r = c;
                        } else {
                            l = c;
                        }
                    }
                    arrow_pos = getp(r).to_point();
                }
            } else {
                ctx.stroke(Line::new(
                    self.transform(Point::new(self.pos[i].x, self.pos[i].y)),
                    self.transform(Point::new(self.pos[j].x, self.pos[j].y))),
                    &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8), WIDTH * self.scale);
                label_pos = self.pos[i].midpoint(self.pos[j]);
                let mut ab = Point::new(self.pos[i].x - self.pos[j].x, self.pos[i].y - self.pos[j].y);
                let lab = ab.distance(Point::new(0., 0.));
                ab.x /= lab; ab.y /= lab;
                ab.x *= RADIUS; ab.y *= RADIUS;

                let getp = |t: f64| {
                    Vec2::new(self.pos[i].x + (self.pos[j].x - self.pos[i].x) * t, self.pos[i].y + (self.pos[j].y - self.pos[i].y) * t)
                };

                {
                    let mut l = 0_f64;
                    let mut r = 1_f64;
                    for _it in 0..50 {
                        let c = (l + r) / 2.;
                        if self.inside_node(&data, j, self.transform(getp(c).to_point())) {
                            r = c;
                        } else {
                            l = c;
                        }
                    }
                    arrow_pos = getp(r).to_point();
                }
            }

            if data.is_edge_info {
                self.draw_edge_info(ctx, data, env, self.transform(label_pos), s);
            }

            if data.directed {
                let ang = if data.draw_arcs {
                    let a1 = Vec2::new(label_pos.x - arrow_pos.x, label_pos.y - arrow_pos.y).normalize().atan2();
                    let a2 = Vec2::new(arrow_pos.x - self.pos[j].x, arrow_pos.y - self.pos[j].y).normalize().atan2();
                    let res1 = (a1 + a2) / 2.;
                    let mut res2 = (a1 + a2) / 2. + std::f64::consts::PI;
                    if res2 >= std::f64::consts::PI * 2. {
                        res2 -= std::f64::consts::PI;
                    }
                    let dist1 = (res1 - a1).abs().min(std::f64::consts::PI * 2. - (res1 - a1).abs());
                    let dist2 = (res2 - a1).abs().min(std::f64::consts::PI * 2. - (res2 - a1).abs());
                    if dist1 < dist2 {
                        res1
                    } else {
                        res2
                    }
                } else {
                    Vec2::new(arrow_pos.x - self.pos[j].x, arrow_pos.y - self.pos[j].y).normalize().atan2()
                };

                let end1 = (arrow_pos.to_vec2() + Vec2::from_angle(ang + ARROW_ANGLE) * ARROW_LEN).to_point();
                let end2 = (arrow_pos.to_vec2() + Vec2::from_angle(ang - ARROW_ANGLE) * ARROW_LEN).to_point();

                let mut path = BezPath::new();
                path.move_to(self.transform(arrow_pos));
                path.line_to(self.transform(end2));
                path.line_to(self.transform(end1));
                path.close_path();

                ctx.fill(&path, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
                ctx.stroke(&path, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8),  WIDTH * self.scale);
            }
        }

        for i in 0..data.g.len() {
            self.draw_vertex(ctx, data, env, i, self.pos[i]);
        }
    }
}

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw graph [flags]

            Draws points

            Flags:
                --help                 Display this message
                --vertex-info, -vi     Add some information to the nodes.
                  -vi=[opt1],[opt2]    Configure options for vertex info
                  -vi=only             Don't show vertex indices, only info
                  -vi=lines            Read info for each vertex from new line (by default
                                       it reads one line and splits it by spaces)
                --edge-info, -ei       Add edge information (on the same line with the
                                       corresponding edge)
                --arcs                 Draw edges as arcs (may be useful with directed edges
                                       in both sides)
                --directed             Directed graph (draw arrows)
        "};
        print!("{}", s);
        return;
    }

    let mut app_data = AppData{
        g: Rc::new(Vec::new()),
        vertex_info: Rc::new(Vec::new()),
        only_vertex_info: false,
        start_from_1: false,
        draw_arcs: false,
        edge_info: Rc::new(Vec::new()),
        is_edge_info: false,
        directed: false,
    };

    let mut is_vertex_info = false;
    let mut vertex_info_lines = false;

    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--vertex-info") || args[i].starts_with("-vi") {
            is_vertex_info = true;
            if args[i].contains("only") {
                app_data.only_vertex_info = true;
            }
            if args[i].contains("lines") {
                vertex_info_lines = true;
            }
        } else if args[i] == "--edge-info" || args[i] == "-ei" {
            app_data.is_edge_info = true;
        } else if args[i] == "--arcs" {
            app_data.draw_arcs = true;
        } else if args[i] == "--directed" {
            app_data.directed = true;
        } else {
            eprintln!("Unknown option \"{}\"", args[i]);
            std::process::exit(1);
        }
        i += 1;
    }

    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    let mut ln = s.trim().split(" ");
    let (n, m) : (usize, usize) = (ln.next().unwrap().parse().unwrap(), ln.next().unwrap().parse().unwrap());

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
    for _i in 0..m {
        s.clear();
        io::stdin().read_line(&mut s).unwrap();
        let mut iter = s.trim().split(" ");
        let mut u = iter.next().unwrap().parse::<usize>().unwrap();
        let mut v = iter.next().unwrap().parse::<usize>().unwrap();
        u -= 1;
        v -= 1;
        edge_info.push(((u, v), if app_data.is_edge_info { iter.collect::<Vec<_>>().join(" ") } else { String::new() }));
        g[u].push(v);
    }

    app_data.edge_info = Rc::new(edge_info);
    app_data.g = Rc::new(g);

    let window = WindowDesc::new(make_layout)
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
            DrawingWidget{
                scale: 100.0,
                center: Point::new(0.0, 0.0),
                last_mouse_pos: Point::new(0.0, 0.0),
                size: Size::new(0.0, 0.0),
                first_time: true,
                pos: Vec::new(),
                moving_vertex: false,
                vertex: 0,
                sizes: Vec::new(),
            }.with_id(drawing_widget_id),
            1.0
        )
        .with_spacer(PADDING)
        .with_child(
            Flex::column()
                .with_child(Checkbox::new("Start from 1").lens(AppData::start_from_1))
                .with_spacer(PADDING)
                .with_child(Checkbox::new("Draw arcs").lens(AppData::draw_arcs))
                .with_spacer(PADDING)
                .with_child(Checkbox::new("Directed").lens(AppData::directed))
                .with_spacer(PADDING)
                .with_child(Button::new("Refresh").on_click(move |ctx: &mut EventCtx, _data, _env| {
                    ctx.submit_command(Command::new(Selector::new("refresh"), (), Target::Widget(drawing_widget_id)));
                }))
                .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::End)
        .padding(PADDING)
}
