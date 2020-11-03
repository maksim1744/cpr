use std::collections::{HashMap};

use std::io::{self};

use indoc::indoc;

use std::rc::Rc;

use druid::widget::prelude::*;
use druid::widget::{Flex, Widget, MainAxisAlignment, CrossAxisAlignment};
use druid::{Size, AppLauncher, WindowDesc, Data, Lens, Color, Rect, Point, WidgetExt, MouseButton};
use druid::kurbo::{Circle, Line};
use druid::piet::{FontFamily, Text, TextLayoutBuilder, TextLayout};

const PADDING: f64 = 8.0;

// for tree nodes
const RADIUS: f64 = 1.0;
const CHILD_SEP: f64 = 0.3;
const LEVEL_SEP: f64 = 1.5;

const BACKGROUND: Color = Color::rgb8(0x30 as u8, 0x30 as u8, 0x30 as u8);

#[derive(Clone, Lens, Data)]
struct AppData {
    g: Rc<Vec<Vec<usize>>>,
}

struct DrawingWidget {
    scale: f64,
    center: Point,
    last_mouse_pos: Point,
    size: Size,
    first_time: bool,
    need_size: Vec<(f64, f64)>,
}

impl DrawingWidget {
    fn transform(&self, mut p: Point) -> Point {
        p.x = (p.x - self.center.x) * self.scale + self.size.width / 2.0;
        p.y = (p.y - self.center.y) * self.scale + self.size.height / 2.0;
        p
    }

    fn init_dfs(&mut self, g: &Vec<Vec<usize>>, v: usize, par: usize) {
        self.need_size[v].1 = RADIUS * 2.0;
        for &k in g[v].iter() {
            if k != par {
                self.init_dfs(g, k, v);
                if self.need_size[v].0 != 0.0 {
                    self.need_size[v].0 += CHILD_SEP;
                }
                self.need_size[v].0 += self.need_size[k].0;
                self.need_size[v].1 = self.need_size[v].1.max(self.need_size[k].1 + RADIUS * 2.0 + LEVEL_SEP);
            }
        }
        self.need_size[v].0 = self.need_size[v].0.max(RADIUS * 2.0);
    }

    fn draw_vertex(&self, ctx: &mut PaintCtx, _data: &AppData, _env: &Env, v: usize, pos: Point) {
        let circle = Circle::new(self.transform(pos), RADIUS * self.scale);
        ctx.fill(circle, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
        let circle = Circle::new(self.transform(pos), RADIUS * self.scale - 2.0);
        ctx.fill(circle, &BACKGROUND);

        let text = ctx.text();
        let layout = text
            .new_text_layout(v.to_string())
            .font(FontFamily::SERIF, 1.0 * self.scale)
            .text_color(Color::rgb8(0xff, 0xff, 0xff))
            // .alignment(TextAlignment::Start)
            .build()
            .unwrap();

        let text_size = layout.size();

        let mut text_pos = self.transform(pos);
        text_pos.x -= text_size.width / 2.0;
        text_pos.y -= text_size.height / 2.0;

        ctx.draw_text(&layout, text_pos);
    }

    fn draw_dfs(&self, ctx: &mut PaintCtx, data: &AppData, env: &Env, v: usize, par: usize, mut pos: Point) -> Point {
        let mut root_pos = pos.clone();
        root_pos.y += RADIUS;

        pos.y += RADIUS * 2.0 + LEVEL_SEP;

        let mut first = true;
        let mut left_child = root_pos.x + self.need_size[v].0 / 2.0;
        let mut right_child = root_pos.x + self.need_size[v].0 / 2.0;

        let mut childs = Vec::new();

        for &k in data.g[v].iter() {
            if k != par {
                let p = self.draw_dfs(ctx, data, env, k, v, pos);
                childs.push((k, p.clone()));
                if first {
                    left_child = p.x;
                    first = false;
                }
                right_child = p.x;
                pos.x += self.need_size[k].0 + CHILD_SEP;
            }
        }

        root_pos.x = (left_child + right_child) / 2.0;

        for &(k, point) in childs.iter() {
            ctx.stroke(Line::new(self.transform(point), self.transform(root_pos)), &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8), 2.0);
            self.draw_vertex(ctx, data, env, k, point);
        }

        root_pos
    }
}

impl Widget<AppData> for DrawingWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut AppData, _env: &Env) {
        match event {
            Event::MouseMove(e) => {
                if e.buttons.contains(MouseButton::Left) {
                    self.center.x -= (e.pos.x - self.last_mouse_pos.x) / self.scale;
                    self.center.y -= (e.pos.y - self.last_mouse_pos.y) / self.scale;
                    self.last_mouse_pos = e.pos;
                    ctx.request_paint();
                }
            },
            Event::Wheel(e) => {
                self.scale = self.scale * 0.01_f64.max(1.1_f64.powf(-e.wheel_delta.y / 25.0));
                ctx.request_paint();
            },
            Event::MouseDown(e) => {
                self.last_mouse_pos = e.pos.clone();
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, env: &Env) {
        let size: Size = ctx.size();
        self.size = size.clone();

        ctx.fill(Rect::from_origin_size(Point{x: 0.0, y: 0.0}, size), &BACKGROUND);

        if self.first_time {
            self.need_size = vec![(0.0, 0.0); data.g.len()];
            self.init_dfs(&data.g, 0 as usize, usize::MAX);
            self.scale = size.height / self.need_size[0].1 * 0.8;
            self.first_time = false;
        }
        let root_pos = self.draw_dfs(ctx, data, env, 0 as usize, usize::MAX, Point{ x: -self.need_size[0].0 / 2.0, y: -self.need_size[0].1 / 2.0 });
        self.draw_vertex(ctx, data, env, 0, root_pos);
    }
}

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw tree [flags]

            Draws points

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }


    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    let n: usize = s.trim().parse().unwrap();
    let mut g: Vec<Vec<usize>> = vec![Vec::new(); n];
    for _i in 0..n-1 {
        s.clear();
        io::stdin().read_line(&mut s).unwrap();
        let mut iter = s.trim().split(" ").map(|x| x.parse::<usize>().unwrap());
        let mut u = iter.next().unwrap();
        let mut v = iter.next().unwrap();
        u -= 1;
        v -= 1;
        g[u].push(v);
        g[v].push(u);
    }

    let window = WindowDesc::new(make_layout)
        .window_size(Size {
            width: 800.0,
            height: 600.0,
        })
        .resizable(true)
        .title("Drawing");
    AppLauncher::with_window(window)
        .launch(AppData{
            g: Rc::new(g),
        })
        .expect("launch failed");
}

fn make_layout() -> impl Widget<AppData> {
    Flex::row()
        .with_flex_child(
            DrawingWidget{
                scale: 100.0,
                center: Point::new(0.0, 0.0),
                last_mouse_pos: Point::new(0.0, 0.0),
                size: Size::new(0.0, 0.0),
                first_time: true,
                need_size: Vec::new(),
            },
            1.0
        )
        .with_spacer(PADDING)
        .with_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::End)
        .padding(PADDING)
}
