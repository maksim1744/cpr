use std::collections::HashMap;
use std::thread;

use std::io::{self};

use indoc::indoc;

use std::sync::{Arc, Mutex};

use druid::kurbo::{Circle, Line};
use druid::widget::prelude::*;
use druid::widget::{Checkbox, CrossAxisAlignment, Flex, MainAxisAlignment, Widget};
use druid::{AppLauncher, Color, Data, Lens, MouseButton, Point, Rect, Size, WidgetExt, WindowDesc};

const PADDING: f64 = 8.0;

#[derive(Clone, Lens, Data)]
struct AppData {
    points: Arc<Mutex<Vec<(f64, f64)>>>,
    mouse: Point,
    show_plus_target: bool,
    show_x_target: bool,
}

struct DrawingWidget {
    scale: f64,
    center: Point,
    last_mouse_pos: Point,
}

impl Widget<AppData> for DrawingWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppData, _env: &Env) {
        match event {
            Event::MouseMove(e) => {
                data.mouse = e.pos.clone();
                if e.buttons.contains(MouseButton::Left) {
                    self.center.x -= (e.pos.x - self.last_mouse_pos.x) / self.scale;
                    self.center.y -= (e.pos.y - self.last_mouse_pos.y) / self.scale;
                    self.last_mouse_pos = e.pos;
                }
            }
            Event::Wheel(e) => {
                self.scale = self.scale * 0.01_f64.max(1.1_f64.powf(-e.wheel_delta.y / 25.0));
                ctx.request_paint();
            }
            Event::MouseDown(e) => {
                self.last_mouse_pos = e.pos.clone();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &AppData, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(&mut self, _layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &AppData, _env: &Env) -> Size {
        Size {
            width: bc.max().width,
            height: bc.max().height,
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let size: Size = ctx.size();

        let transform = |mut p: Point| -> Point {
            p.x = (p.x - self.center.x) * self.scale + size.width / 2.0;
            p.y = (p.y - self.center.y) * self.scale + size.height / 2.0;
            p
        };
        let inv_transform = |mut p: Point| -> Point {
            p.x = (p.x - size.width / 2.0) / self.scale + self.center.x;
            p.y = (p.y - size.height / 2.0) / self.scale + self.center.y;
            p
        };

        let down_left = inv_transform(Point { x: 0.0, y: 0.0 });
        let up_right = inv_transform(Point {
            x: size.width,
            y: size.height,
        });

        ctx.fill(
            Rect::from_origin_size(Point { x: 0.0, y: 0.0 }, size),
            &Color::rgb8(0x30 as u8, 0x30 as u8, 0x30 as u8),
        );

        let cell_size = self.scale;

        let grid_color = Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8);

        ctx.stroke(
            Line::new(
                transform(Point { x: down_left.x, y: 0.0 }),
                transform(Point { x: up_right.x, y: 0.0 }),
            ),
            &grid_color,
            2.0,
        );
        ctx.stroke(
            Line::new(
                transform(Point { x: 0.0, y: down_left.y }),
                transform(Point { x: 0.0, y: up_right.y }),
            ),
            &grid_color,
            2.0,
        );

        let mut koef_grid = 1.0;
        while koef_grid * cell_size < 5.0 {
            koef_grid *= 5.0;
        }

        let mut y = transform(Point {
            x: 0.0,
            y: ((down_left.y / koef_grid).ceil() * koef_grid),
        })
        .y;
        while y < size.height {
            ctx.stroke(
                Line::new(Point { x: 0.0, y: y }, Point { x: size.width, y: y }),
                &grid_color,
                0.3,
            );
            y += cell_size * koef_grid;
        }

        let mut x = transform(Point {
            x: ((down_left.x / koef_grid).ceil() * koef_grid),
            y: 0.0,
        })
        .x;
        while x < size.width {
            ctx.stroke(
                Line::new(Point { x: x, y: 0.0 }, Point { x: x, y: size.height }),
                &grid_color,
                0.3,
            );
            x += cell_size * koef_grid;
        }

        for (x, y) in data.points.lock().unwrap().iter().map(|&(x, y)| (x as f64, -y as f64)) {
            let p = transform(Point { x: x, y: y });
            if p.x > 0.0 && p.y > 0.0 && p.x < size.width && p.y < size.height {
                let point = Circle::new(p, 5.0);
                ctx.fill(point, &Color::rgb8(0xff as u8, 0xff as u8, 0xff as u8));
            }
        }

        let target_color = Color::rgb8(0xff as u8, 0 as u8, 0 as u8);
        if data.show_plus_target
            && data.mouse.x > 0.0
            && data.mouse.y > 0.0
            && data.mouse.x < size.width
            && data.mouse.y < size.height
        {
            ctx.stroke(
                Line::new(
                    Point {
                        x: 0.0,
                        y: data.mouse.y,
                    },
                    Point {
                        x: size.width,
                        y: data.mouse.y,
                    },
                ),
                &target_color,
                1.5,
            );
            ctx.stroke(
                Line::new(
                    Point {
                        x: data.mouse.x,
                        y: 0.0,
                    },
                    Point {
                        x: data.mouse.x,
                        y: size.height,
                    },
                ),
                &target_color,
                1.5,
            );
        }

        if data.show_x_target
            && data.mouse.x > 0.0
            && data.mouse.y > 0.0
            && data.mouse.x < size.width
            && data.mouse.y < size.height
        {
            let down_left = data.mouse.x.min(data.mouse.y);
            ctx.stroke(
                Line::new(
                    Point {
                        x: data.mouse.x - down_left,
                        y: data.mouse.y - down_left,
                    },
                    data.mouse,
                ),
                &target_color,
                1.0,
            );

            let up_right = (size.width - data.mouse.x).min(size.height - data.mouse.y);
            ctx.stroke(
                Line::new(
                    Point {
                        x: data.mouse.x + up_right,
                        y: data.mouse.y + up_right,
                    },
                    data.mouse,
                ),
                &target_color,
                1.0,
            );

            let up_left = data.mouse.x.min(size.height - data.mouse.y);
            ctx.stroke(
                Line::new(
                    Point {
                        x: data.mouse.x - up_left,
                        y: data.mouse.y + up_left,
                    },
                    data.mouse,
                ),
                &target_color,
                1.0,
            );

            let down_right = (size.width - data.mouse.x).min(data.mouse.y);
            ctx.stroke(
                Line::new(
                    Point {
                        x: data.mouse.x + down_right,
                        y: data.mouse.y - down_right,
                    },
                    data.mouse,
                ),
                &target_color,
                1.0,
            );
        }
    }
}

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw points [flags]

            Draws points

            Flags:
                --help              Display this message
                --non               Don't read n, just read points
        "};
        print!("{}", s);
        return;
    }

    let mut i = 0;
    let mut non = false;

    while i < args.len() {
        if args[i] == "-non" {
            non = true;
        } else {
            eprintln!("Unknown arg {}", args[i]);
            std::process::exit(1);
        }
        i += 1;
    }

    let ptr = Arc::new(Mutex::new(Vec::<(f64, f64)>::new()));
    let thread_ptr = ptr.clone();

    let handle = thread::spawn(move || {
        let window = WindowDesc::new(make_layout())
            .window_size(Size {
                width: 800.0,
                height: 600.0,
            })
            .resizable(true)
            .title("Drawing");
        AppLauncher::with_window(window)
            .launch(AppData {
                points: thread_ptr,
                mouse: Point { x: 0.0, y: 0.0 },
                show_plus_target: true,
                show_x_target: true,
            })
            .expect("launch failed");
    });

    let mut s = String::new();
    let n: i32;
    if !non {
        io::stdin().read_line(&mut s).unwrap();
        n = s.trim().split(" ").next().unwrap().parse().unwrap();
    } else {
        n = i32::MAX;
    }
    for _i in 0..n {
        s.clear();
        if let Ok(0) = io::stdin().read_line(&mut s) {
            break;
        }
        let mut iter = s.trim().split(" ").map(|x| x.parse::<f64>().unwrap());
        ptr.lock().unwrap().push((iter.next().unwrap(), iter.next().unwrap()));
    }

    handle.join().unwrap();
}

fn make_layout() -> impl Widget<AppData> {
    Flex::row()
        .with_flex_child(
            DrawingWidget {
                scale: 100.0,
                center: Point { x: 0.0, y: 0.0 },
                last_mouse_pos: Point { x: 0.0, y: 0.0 },
            },
            1.0,
        )
        .with_spacer(PADDING)
        .with_child(
            Flex::column()
                .with_child(Checkbox::new("Show + target").lens(AppData::show_plus_target))
                .with_spacer(PADDING)
                .with_child(Checkbox::new("Show x target").lens(AppData::show_x_target))
                .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::End)
        .padding(PADDING)
}
