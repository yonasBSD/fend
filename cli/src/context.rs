use std::time;

use tokio::sync::RwLock;

use crate::{config, exchange_rates};

pub struct HintInterrupt {
	start: time::Instant,
	duration: time::Duration,
}

impl fend_core::Interrupt for HintInterrupt {
	fn should_interrupt(&self) -> bool {
		time::Instant::now().duration_since(self.start) >= self.duration
	}
}

impl Default for HintInterrupt {
	fn default() -> Self {
		Self {
			start: time::Instant::now(),
			duration: time::Duration::from_millis(20),
		}
	}
}

pub struct InnerCtx {
	core_ctx: fend_core::Context,

	// true if the user typed some partial input, false otherwise
	input_typed: bool,
}

impl InnerCtx {
	pub fn new(config: &config::Config) -> Self {
		let mut res = Self {
			core_ctx: fend_core::Context::new(),
			input_typed: false,
		};
		if config.coulomb_and_farad {
			res.core_ctx.use_coulomb_and_farad();
		}
		for custom_unit in &config.custom_units {
			res.core_ctx.define_custom_unit_v1(
				&custom_unit.singular,
				&custom_unit.plural,
				&custom_unit.definition,
				&custom_unit.attribute.to_fend_core(),
			);
		}
		res.core_ctx
			.set_decimal_separator_style(config.decimal_separator);
		let exchange_rate_handler = exchange_rates::ExchangeRateHandler {
			enable_internet_access: config.enable_internet_access,
			source: config.exchange_rate_source,
			max_age: config.exchange_rate_max_age,
		};
		res.core_ctx
			.set_exchange_rate_handler_v2(exchange_rate_handler);
		res
	}
}

#[derive(Clone)]
pub struct Context<'a> {
	ctx: &'a RwLock<InnerCtx>,
}

impl<'a> Context<'a> {
	pub fn new(ctx: &'a RwLock<InnerCtx>) -> Self {
		Self { ctx }
	}

	pub async fn eval(
		&self,
		line: &str,
		echo_result: bool,
		int: &impl fend_core::Interrupt,
	) -> Result<fend_core::FendResult, String> {
		use rand::SeedableRng;

		let mut ctx_borrow = self.ctx.write().await;
		ctx_borrow
			.core_ctx
			.set_random_u32_trait(Random(rand::rngs::StdRng::from_os_rng()));
		ctx_borrow.core_ctx.set_output_mode_terminal();
		ctx_borrow.core_ctx.set_echo_result(echo_result);
		ctx_borrow.input_typed = false;
		tokio::task::block_in_place(|| {
			fend_core::evaluate_with_interrupt(line, &mut ctx_borrow.core_ctx, int)
		})
	}

	pub async fn eval_hint(&self, line: &str) -> fend_core::FendResult {
		let mut ctx_borrow = self.ctx.write().await;
		ctx_borrow.core_ctx.set_output_mode_terminal();
		ctx_borrow.input_typed = !line.is_empty();
		let int = HintInterrupt::default();
		tokio::task::block_in_place(|| {
			fend_core::evaluate_preview_with_interrupt(line, &ctx_borrow.core_ctx, &int)
		})
	}

	pub async fn serialize(&self) -> Result<Vec<u8>, String> {
		let mut result = vec![];
		self.ctx
			.read()
			.await
			.core_ctx
			.serialize_variables(&mut result)?;
		Ok(result)
	}

	pub async fn get_input_typed(&self) -> bool {
		self.ctx.read().await.input_typed
	}
}

struct Random(rand::rngs::StdRng);

impl fend_core::random::RandomSource for Random {
	fn get_random_u32(&mut self) -> u32 {
		use rand::Rng;
		self.0.random()
	}
}
