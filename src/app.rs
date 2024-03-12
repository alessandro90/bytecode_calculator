#[cfg(not(feature = "gui"))]
mod terminal {
    use std::io::{self, Write};

    use crate::{
        compiler::{Compile, Compiler, Error as CompilerError},
        lexer::Lexer,
        vm::{Error as VMError, VirtualMachine},
    };

    #[derive(Debug, Clone)]
    enum ApplicationError {
        CompileError(CompilerError),
        VirtualmachineError(VMError),
    }

    impl std::fmt::Display for ApplicationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for ApplicationError {}

    impl From<CompilerError> for ApplicationError {
        fn from(value: CompilerError) -> Self {
            Self::CompileError(value)
        }
    }

    impl From<VMError> for ApplicationError {
        fn from(value: VMError) -> Self {
            Self::VirtualmachineError(value)
        }
    }

    fn run_file(src: &[u8]) -> Result<f64, ApplicationError> {
        let mut lexer = Lexer::new(src);
        let mut compiler = Compiler::default();
        compiler.compile(&mut lexer)?;
        let mut vm = VirtualMachine::default();
        vm.interpret(compiler.opcodes()).map_err(|e| e.into())
    }

    fn run_repl() -> ! {
        let mut input = String::new();
        let mut compiler = Compiler::default();
        let mut vm = VirtualMachine::default();
        loop {
            print!(">> ");
            io::stdout().flush().unwrap();
            input.clear();
            if io::stdin().read_line(&mut input).is_err() {
                continue;
            }
            if input == "\n" || input == "\r\n" {
                continue;
            }
            let bytes = input.as_bytes();
            let mut lexer = Lexer::new(bytes);
            if let Err(e) = compiler.compile(&mut lexer) {
                eprintln!("Compiler error: {}", e);
                compiler.reset();
                continue;
            }
            let ans = match vm.interpret(compiler.opcodes()) {
                Ok(value) => {
                    println!("$ {}", value);
                    Some(value)
                }
                Err(e) => {
                    eprintln!("Virtual machine error: {}", e);
                    None
                }
            };
            vm.reset(ans);
            compiler.reset();
        }
    }

    pub fn run() -> std::process::ExitCode {
        match std::env::args().nth(1) {
            Some(src_path) => {
                let src =
                    std::fs::read(src_path).unwrap_or_else(|e| panic!("Cannot read file {}", e));
                match run_file(&src) {
                    Ok(res) => {
                        println!("Result of computation: {}", res);
                        std::process::ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::ExitCode::FAILURE
                    }
                }
            }
            None => run_repl(),
        }
    }
}

#[cfg(not(feature = "gui"))]
pub use terminal::*;

#[cfg(feature = "gui")]
mod gui {

    use eframe::egui;

    use crate::{
        compiler::{Compile, Compiler},
        lexer::Lexer,
        vm::VirtualMachine,
    };

    #[derive(Default)]
    struct App {
        expression: String,
        result: String,
        compiler: Compiler,
        vm: VirtualMachine,
    }

    impl eframe::App for App {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Calculator");
                ui.label(&self.expression);
                ui.label(&self.result);
                self.buttons(ui);
            });
        }
    }

    impl App {
        fn solve(&mut self) {
            let mut lexer = Lexer::new(self.expression.as_bytes());
            match self.compiler.compile(&mut lexer) {
                Ok(_) => match self.vm.interpret(self.compiler.opcodes()) {
                    Ok(r) => {
                        self.result = r.to_string();
                        self.compiler.reset();
                        self.vm.reset(Some(r));
                    }
                    Err(e) => {
                        self.result = e.to_string();
                        self.compiler.reset();
                        self.vm.reset(None);
                    }
                },
                Err(e) => self.result = e.to_string(),
            };
            self.expression.clear();
        }

        fn draw_number_row(&mut self, ui: &mut egui::Ui, nums: [&'static str; 3]) {
            for num in nums {
                if ui.add(single_char_btn(num)).clicked() {
                    self.expression.push_str(num);
                }
            }
        }

        fn buttons(&mut self, ui: &mut egui::Ui) {
            ui.style_mut().spacing.item_spacing = egui::Vec2::new(1.0, 1.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.add(single_char_btn("(")).clicked() {
                            self.expression.push('(');
                        }
                        if ui.add(single_char_btn(")")).clicked() {
                            self.expression.push(')');
                        }
                        if ui.add(large_btn("Del")).clicked() {
                            self.expression.pop();
                        }
                    });
                    egui::Grid::new("numbers_grid_id")
                        .spacing([1.0, 1.0])
                        .min_col_width(20.0)
                        .show(ui, |ui| {
                            self.draw_number_row(ui, ["1", "2", "3"]);
                            if ui.add(single_char_btn("+")).clicked() {
                                self.expression.push('+');
                            }
                            ui.end_row();
                            self.draw_number_row(ui, ["4", "5", "6"]);
                            if ui.add(single_char_btn("-")).clicked() {
                                self.expression.push('-');
                            }
                            ui.end_row();
                            self.draw_number_row(ui, ["7", "8", "9"]);
                            if ui.add(single_char_btn("*")).clicked() {
                                self.expression.push('*');
                            }
                            ui.end_row();
                            if ui.add(single_char_btn("0")).clicked() {
                                self.expression.push('0');
                            }
                            if ui.add(single_char_btn(".")).clicked() {
                                self.expression.push('.');
                            }
                            if ui.add(single_char_btn("=")).clicked() {
                                self.solve();
                            }
                            if ui.add(single_char_btn("/")).clicked() {
                                self.expression.push('/');
                            }
                            ui.end_row();
                        });
                });
                ui.vertical(|ui| {
                    for f in ["cos", "sin", "sqrt", "log", "ans"] {
                        if ui.add(large_btn(f)).clicked() {
                            self.expression.push_str(f);
                        }
                    }
                });
            });
        }
    }

    fn single_char_btn(n: &str) -> egui::Button {
        egui::Button::new(n).min_size(egui::Vec2::new(BTN_WIDTH, BTN_HEGHT))
    }

    fn large_btn(n: &str) -> egui::Button {
        egui::Button::new(n).min_size(egui::Vec2::new(BTN_LARGE_WIDTH, BTN_HEGHT))
    }

    const BTN_HEGHT: f32 = 20.0;
    const BTN_WIDTH: f32 = 20.0;
    const BTN_LARGE_WIDTH: f32 = 2. * BTN_WIDTH;
    const W_WIDTH: f32 = 6. * BTN_WIDTH + 20.;
    const W_HEIGHT: f32 = 200.0;

    pub fn run() -> std::process::ExitCode {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([W_WIDTH, W_HEIGHT]),
            ..Default::default()
        };
        match eframe::run_native("Calculator", options, Box::new(|_| Box::<App>::default())) {
            Ok(_) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Cannot run application: {}", e);
                std::process::ExitCode::FAILURE
            }
        }
    }
}

#[cfg(feature = "gui")]
pub use gui::*;
