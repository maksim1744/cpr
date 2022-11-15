use std::collections::HashMap;

use std::io::{self};

use indoc::indoc;

use std::rc::Rc;

use druid::kurbo::Line;
use druid::piet::{FontFamily, Text, TextLayout, TextLayoutBuilder};
use druid::widget::prelude::*;
use druid::widget::{CrossAxisAlignment, Flex, MainAxisAlignment, Widget};
use druid::{AppLauncher, Color, Data, Lens, MouseButton, Point, Rect, Size, WidgetExt, WindowDesc};

const PADDING: f64 = 8.0;

const WIDTH: f64 = 0.01;
const FONT_SIZE: f64 = 0.8;

const BACKGROUND: Color = Color::rgb8(0x30 as u8, 0x30 as u8, 0x30 as u8);

#[derive(Clone, Lens, Data)]
struct AppData {
    mt: Rc<Vec<Vec<String>>>,
    chars: bool,
    width: usize,
    draw_as_wires: [bool; 6],
}

struct DrawingWidget {
    scale: f64,
    center: Point,
    last_mouse_pos: Point,
    size: Size,
    first_time: bool,
    cell_width: f64,
}

impl DrawingWidget {
    fn transform(&self, mut p: Point) -> Point {
        p.x = (p.x - self.center.x) * self.scale + self.size.width / 2.0;
        p.y = (p.y - self.center.y) * self.scale + self.size.height / 2.0;
        p
    }

    fn calculate_cell_width(&self, ctx: &mut PaintCtx, data: &AppData) -> f64 {
        if data.chars {
            return 1.0;
        }
        let mut res: f64 = 0.0;
        for row in data.mt.iter() {
            for item in row.iter() {
                let text = ctx.text();
                let layout = text
                    .new_text_layout(item.clone())
                    .font(FontFamily::MONOSPACE, FONT_SIZE * self.scale)
                    .text_color(Color::rgb8(0xff, 0xff, 0xff))
                    // .alignment(TextAlignment::Start)
                    .build()
                    .unwrap();

                let text_size = layout.size();
                res = res.max(text_size.width);
            }
        }
        res / self.scale + 0.5
    }

    fn refresh(&mut self, ctx: &mut PaintCtx, data: &AppData) {
        let size: Size = ctx.size();
        self.cell_width = self.calculate_cell_width(ctx, data);

        let width = self.cell_width * data.width as f64;
        let height = data.mt.len() as f64;
        self.center = Point::new(width / 2.0, height / 2.0);
        self.scale = (size.width / width).min(size.height / height) * 0.9;
        self.first_time = false;
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
            }
            Event::Wheel(e) => {
                self.scale = self.scale * 0.01_f64.max(1.1_f64.powf(-e.wheel_delta.y / 25.0));
                ctx.request_paint();
            }
            Event::MouseDown(e) => {
                self.last_mouse_pos = e.pos.clone();
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let size: Size = ctx.size();
        self.size = size.clone();

        ctx.fill(Rect::from_origin_size(Point { x: 0.0, y: 0.0 }, size), &BACKGROUND);

        if self.first_time {
            self.refresh(ctx, data);
        }

        for (i, row) in data.mt.iter().enumerate() {
            for (j, item) in row.iter().enumerate() {
                let mut text_pos = self.transform(Point::new((j as f64 + 0.5) * self.cell_width, i as f64 + 0.5));

                if text_pos.x < -self.cell_width * self.scale
                    || text_pos.y < -0.5 * self.scale
                    || text_pos.x > size.width + self.cell_width * self.scale
                    || text_pos.y > size.height + 0.5 * self.scale
                {
                    continue;
                }

                let text = ctx.text();
                let layout = text
                    .new_text_layout(item.clone())
                    .font(FontFamily::MONOSPACE, FONT_SIZE * self.scale)
                    .text_color(Color::rgb8(0xff, 0xff, 0xff))
                    // .alignment(TextAlignment::Start)
                    .build()
                    .unwrap();

                let text_size = layout.size();

                text_pos.x -= text_size.width / 2.0;
                text_pos.y -= text_size.height / 2.0;

                ctx.draw_text(&layout, text_pos);
            }
        }

        for i in 0..data.width + 1 {
            ctx.stroke(
                Line::new(
                    self.transform(Point::new(i as f64 * self.cell_width, 0.0)),
                    self.transform(Point::new(i as f64 * self.cell_width, data.mt.len() as f64)),
                ),
                &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8),
                WIDTH * self.scale,
            );
        }
        for i in 0..data.mt.len() + 1 {
            ctx.stroke(
                Line::new(
                    self.transform(Point::new(0.0, i as f64)),
                    self.transform(Point::new(self.cell_width * data.width as f64, i as f64)),
                ),
                &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8),
                WIDTH * self.scale,
            );
        }
    }
}

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw matrix [flags]

            Draws matrix or a cell field.

            Flags:
                --help                 Display this message
                --chars, -c            Make a cell for each char, not each word
        "};
        print!("{}", s);
        return;
    }

    let mut app_data = AppData {
        mt: Rc::new(Vec::new()),
        chars: false,
        width: 0,
        draw_as_wires: [false; 6],
    };

    let mut i = 0;
    while i < args.len() {
        if args[i] == "--chars" || args[i] == "-c" {
            app_data.chars = true;
        } else {
            eprintln!("Unknown option \"{}\"", args[i]);
            std::process::exit(1);
        }
        i += 1;
    }

    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    let n: usize = s.trim().split(" ").next().unwrap().parse().unwrap();

    let mut mt: Vec<Vec<String>> = vec![Vec::new(); n];

    for i in 0..n {
        s.clear();
        io::stdin().read_line(&mut s).unwrap();
        if app_data.chars {
            mt[i] = s.trim().chars().map(|c| String::from(c)).collect::<Vec<_>>();
        } else {
            mt[i] = s.trim().split_whitespace().map(String::from).collect::<Vec<_>>();
        }
        app_data.width = app_data.width.max(mt[i].len());
    }

    app_data.mt = Rc::new(mt);

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
                cell_width: 0.0,
            }
            .with_id(drawing_widget_id),
            1.0,
        )
        .with_spacer(PADDING)
        .with_child(
            Flex::column(), // .with_child(
                            //     Flex::row()
                            //         .with_child(
                            //             Flex::column()
                            // .with_child(Checkbox::new("Show + target").lens(AppData::draw_as_wires[0]))
                            // .with_spacer(PADDING)
                            // .with_child(Checkbox::new("Show x target").lens(AppData::show_x_target))
                            // .cross_axis_alignment(CrossAxisAlignment::Start),
                            //         )
                            // )
                            // .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::End)
        .padding(PADDING)
}
