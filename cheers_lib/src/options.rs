#[derive(Clone, Copy)]
pub struct SearchOptions {
    pub threads: usize,
    pub tt_size_mb: usize,
    pub nmp_depth: i8,
    pub nmp_const_reduction: i8,
    pub nmp_improving_reduction: i8,
    pub nmp_linear_divisor: i8,
    pub see_pruning_depth: i8,
    pub see_capture_margin: i16,
    pub see_quiet_margin: i16,
    pub pvs_fulldepth: i8,
    pub delta_pruning_margin: i16,
    pub fp_margin_1: i16,
    pub fp_margin_2: i16,
    pub fp_margin_3: i16,
    pub rfp_margin: i16,
    pub rfp_improving_margin: i16,
    pub lmp_depth: i8,
    pub lmp_const: f32,
    pub lmp_const_scale: f32,
    pub lmp_improving: f32,
    pub lmp_improving_scale: f32,
    pub lmr_const: f32,
    pub lmr_const_divisor: f32,
    pub lmr_improving: f32,
    pub lmr_improving_divisor: f32,
    pub iir_depth: i8,
}

pub const NMP_DEPTH: i8 = 1;
pub const NMP_CONST_REDUCTION: i8 = 2;
pub const NMP_IMPROVING_REDUCTION: i8 = 0;
pub const NMP_LINEAR_DIVISOR: i8 = 2;
pub const SEE_PRUNING_DEPTH: i8 = 10;
pub const SEE_CAPTURE_MARGIN: i16 = -45;
pub const SEE_QUIET_MARGIN: i16 = -50;
pub const PVS_FULLDEPTH: i8 = 1;
pub const DELTA_PRUNING_MARGIN: i16 = 185;
pub const FP_MARGIN_1: i16 = 100;
pub const FP_MARGIN_2: i16 = 313;
pub const FP_MARGIN_3: i16 = 547;
pub const RFP_MARGIN: i16 = 205;
pub const RFP_IMPROVING_MARGIN: i16 = 0;
pub const LMP_DEPTH: i8 = 10;
pub const LMP_CONST: f32 = 2.5;
pub const LMP_CONST_SCALE: f32 = 0.625;
pub const LMP_IMPROVING: f32 = 4.22;
pub const LMP_IMPROVING_SCALE: f32 = 1.414;
pub const LMR_CONST: f32 = 1.5;
pub const LMR_DIVISOR: f32 = 1.75;
pub const LMR_IMPROVING_CONST: f32 = 1.5;
pub const LMR_IMPROVING_DIVISOR: f32 = 1.75;
pub const IIR_DEPTH: i8 = 7;

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            threads: 1,
            tt_size_mb: 8,
            nmp_depth: NMP_DEPTH,
            nmp_const_reduction: NMP_CONST_REDUCTION,
            nmp_improving_reduction: NMP_IMPROVING_REDUCTION,
            nmp_linear_divisor: NMP_LINEAR_DIVISOR,
            see_pruning_depth: SEE_PRUNING_DEPTH,
            see_capture_margin: SEE_CAPTURE_MARGIN,
            see_quiet_margin: SEE_QUIET_MARGIN,
            pvs_fulldepth: PVS_FULLDEPTH,
            delta_pruning_margin: DELTA_PRUNING_MARGIN,
            fp_margin_1: FP_MARGIN_1,
            fp_margin_2: FP_MARGIN_2,
            fp_margin_3: FP_MARGIN_3,
            rfp_margin: RFP_MARGIN,
            rfp_improving_margin: RFP_IMPROVING_MARGIN,
            lmp_depth: LMP_DEPTH,
            lmp_const: LMP_CONST,
            lmp_const_scale: LMP_CONST_SCALE,
            lmp_improving: LMP_IMPROVING,
            lmp_improving_scale: LMP_IMPROVING_SCALE,
            iir_depth: IIR_DEPTH,
            lmr_const: LMR_CONST,
            lmr_const_divisor: LMR_DIVISOR,
            lmr_improving: LMR_IMPROVING_CONST,
            lmr_improving_divisor: LMR_IMPROVING_DIVISOR,
        }
    }
}
