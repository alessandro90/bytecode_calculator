use crate::{compiler::Error as CompilerError, vm::Error as VMError};

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

#[cfg(not(feature = "gui"))]
mod terminal {
    use super::ApplicationError;
    use std::io::{self, Write};

    use crate::{
        compiler::{Compile, Compiler},
        lexer::Lexer,
        vm::VirtualMachine,
    };

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
    use std::{collections::VecDeque, slice::from_raw_parts, str::from_utf8_unchecked};

    use crate::{
        compiler::{Compile, Compiler},
        lexer::Lexer,
        vm::VirtualMachine,
    };

    use super::ApplicationError;

    #[derive(Default)]
    struct App {
        expression: String,
        prev_expressions: VecDeque<String>,
        expression_index: Option<usize>,
        result: String,
        compiler: Compiler,
        vm: VirtualMachine,
    }

    impl eframe::App for App {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Calculator");
                let (ptr, size) = self.expression_to_compute();
                let s = unsafe { from_raw_parts(ptr, size) };
                ui.label(unsafe { from_utf8_unchecked(s) });
                ui.label(&self.result);
                self.buttons(ui);
            });
        }
    }

    impl App {
        // work around lifetime limitations. If I return a &str here
        // the borrow checker does not allow the code in solve (it sees
        // a mutable and a non mutable borrow at the same time).
        // Otherwise I should just inline the function body. The code
        // would be allowed, but duplicated
        fn expression_to_compute(&self) -> (*const u8, usize) {
            match self.expression_index {
                Some(i) => {
                    let s = self.prev_expressions[self.prev_expressions.len() - i - 1].as_bytes();
                    (s.as_ptr(), s.len())
                }
                None => (self.expression.as_ptr(), self.expression.len()),
            }
        }

        fn solve(&mut self) {
            let (ptr, size) = self.expression_to_compute();
            let s = unsafe { from_raw_parts(ptr, size) };
            let mut lexer = Lexer::new(s);
            let res = self
                .compiler
                .compile(&mut lexer)
                .map_err(ApplicationError::from)
                .and_then(|_| {
                    self.vm
                        .interpret(self.compiler.opcodes())
                        .map_err(|e| e.into())
                });
            match res {
                Ok(r) => {
                    self.result = format!("{:+e}", r);
                    let expr = unsafe { from_utf8_unchecked(s) };
                    if self.expression_index.is_none()
                        && Some(expr) != self.prev_expressions.back().map(|x| x.as_str())
                    {
                        self.prev_expressions.push_back(expr.to_owned());
                    }
                    if self.prev_expressions.len() >= 10 {
                        self.prev_expressions.pop_front();
                    }
                    self.compiler.reset();
                    self.vm.reset(Some(r));
                }
                Err(e) => {
                    self.result = e.to_string();
                    self.compiler.reset();
                    self.vm.reset(None);
                }
            };
            self.expression.clear();
        }

        fn draw_number_row(&mut self, ui: &mut egui::Ui, nums: [&'static str; 3]) {
            for num in nums {
                if ui.add(single_char_btn(num)).clicked() && self.expression_index.is_none() {
                    self.expression.push_str(num);
                }
            }
        }

        fn draw_function(&mut self, ui: &mut egui::Ui, fname: &str) {
            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
                if ui
                    .add(large_btn(fname).fill(egui::Color32::from_rgb(51, 66, 255)))
                    .clicked()
                    && self.expression_index.is_none()
                {
                    self.expression.push_str(fname);
                    self.expression.push('(');
                }
            });
        }

        fn buttons(&mut self, ui: &mut egui::Ui) {
            ui.style_mut().spacing.item_spacing = egui::Vec2::new(1.0, 1.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(single_char_btn("(")).clicked() && self.expression_index.is_none() {
                        self.expression.push('(');
                    }
                    if ui.add(single_char_btn(")")).clicked() && self.expression_index.is_none() {
                        self.expression.push(')');
                    }
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                        if ui
                            .add(large_btn("=").fill(egui::Color32::from_rgb(255, 165, 51)))
                            .clicked()
                        {
                            self.solve();
                        }
                    });
                    self.draw_function(ui, "cos");
                    self.draw_function(ui, "sqrt");
                });
                ui.horizontal(|ui| {
                    self.draw_number_row(ui, ["1", "2", "3"]);
                    if ui.add(single_char_btn("+")).clicked() && self.expression_index.is_none() {
                        self.expression.push('+');
                    }
                    self.draw_function(ui, "sin");
                    self.draw_function(ui, "log");
                });

                ui.horizontal(|ui| {
                    self.draw_number_row(ui, ["4", "5", "6"]);
                    if ui.add(single_char_btn("-")).clicked() && self.expression_index.is_none() {
                        self.expression.push('-');
                    }
                    self.draw_function(ui, "pow");
                    if ui.add(large_btn("10^x")).clicked() && self.expression_index.is_none() {
                        self.expression.push('e');
                    }
                });
                ui.horizontal(|ui| {
                    self.draw_number_row(ui, ["7", "8", "9"]);
                    if ui.add(single_char_btn("*")).clicked() && self.expression_index.is_none() {
                        self.expression.push('*');
                    }
                    if ui.add(large_btn("ans")).clicked() && self.expression_index.is_none() {
                        self.expression.push_str("ans");
                    }
                    if ui.add(large_btn("Del")).clicked() && self.expression_index.is_none() {
                        self.expression.pop();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.add(single_char_btn("0")).clicked() && self.expression_index.is_none() {
                        self.expression.push('0');
                    }
                    if ui.add(single_char_btn(".")).clicked() && self.expression_index.is_none() {
                        self.expression.push('.');
                    }
                    if ui.add(single_char_btn(",")).clicked() && self.expression_index.is_none() {
                        self.expression.push(',');
                    }
                    if ui.add(single_char_btn("/")).clicked() && self.expression_index.is_none() {
                        self.expression.push('/');
                    }
                    if ui.add(large_btn("prev")).clicked() {
                        let idx = match self.expression_index {
                            Some(idx) => idx + 1,
                            None => 0,
                        };
                        if self.prev_expressions.len() > idx {
                            self.expression_index = Some(idx);
                        }
                    }
                    if ui.add(large_btn("next")).clicked() {
                        if let Some(idx) = self.expression_index {
                            if idx == 0 {
                                self.expression_index = None;
                            } else {
                                self.expression_index = Some(idx - 1);
                            };
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
    const W_WIDTH: f32 = 8. * BTN_WIDTH + 20.;
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
