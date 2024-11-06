use crate::config::Val;

// Computed at ../examples/gen_seedboxes.rs

pub const POSEIDON2_W16_D5_ROUNDS_F: usize = 8;
pub const POSEIDON2_W16_D5_ROUNDS_P: usize = 14;
pub const POSEIDON2_W16_D5_EXTERNAL_CONSTANTS: [[Val; 16]; 8] = [
    [Val::new(1428922684), Val::new(2022196109), Val::new(1224505130), Val::new(984282662),
        Val::new(1745528643), Val::new(1884925147), Val::new(1845326973), Val::new(976109012),
        Val::new(364320740), Val::new(1169816424), Val::new(1266509633), Val::new(1147500482),
        Val::new(804946803), Val::new(1336891277), Val::new(1923680287), Val::new(1051112063)],
    [Val::new(617202902), Val::new(1328322895), Val::new(809658739), Val::new(728996169),
        Val::new(367124292), Val::new(1183101044), Val::new(2017892963), Val::new(797916161),
        Val::new(1689484235), Val::new(1657723214), Val::new(1725191991), Val::new(607916694),
        Val::new(304711241), Val::new(991633463), Val::new(1341032671), Val::new(1455985172)],
    [Val::new(940327040), Val::new(1836866420), Val::new(1744330360), Val::new(1728313833),
        Val::new(1256787822), Val::new(143243872), Val::new(394906775), Val::new(93462334),
        Val::new(2095314515), Val::new(1438973973), Val::new(1925653183), Val::new(1615496024),
        Val::new(772213231), Val::new(1188568581), Val::new(411016683), Val::new(452512591)],
    [Val::new(913633223), Val::new(1119952228), Val::new(2147150098), Val::new(1631257849),
        Val::new(722026530), Val::new(51210008), Val::new(669586161), Val::new(391858424),
        Val::new(1872572836), Val::new(1530649179), Val::new(1905358042), Val::new(712337723),
        Val::new(273042458), Val::new(143817816), Val::new(2105695752), Val::new(418301610)],
    [Val::new(760850064), Val::new(724582512), Val::new(1175911295), Val::new(1686822328),
        Val::new(1838736009), Val::new(1027362987), Val::new(45299051), Val::new(326225160),
        Val::new(1722439737), Val::new(202954879), Val::new(433482402), Val::new(717784287),
        Val::new(957447280), Val::new(2072056797), Val::new(1476433164), Val::new(1961211085)],
    [Val::new(1402211604), Val::new(2047616321), Val::new(1725105359), Val::new(1403872103),
        Val::new(636199198), Val::new(711763034), Val::new(755524500), Val::new(1146269098),
        Val::new(440942860), Val::new(172467545), Val::new(1346808457), Val::new(680815102),
        Val::new(1145397703), Val::new(493957525), Val::new(1518357280), Val::new(811756323)],
    [Val::new(1599785888), Val::new(384859669), Val::new(1834738991), Val::new(349292068),
        Val::new(1562910107), Val::new(469337841), Val::new(854962023), Val::new(1219794154),
        Val::new(614870544), Val::new(533548718), Val::new(764382489), Val::new(609018108),
        Val::new(1175651676), Val::new(533401582), Val::new(208843075), Val::new(346968022)],
    [Val::new(135087855), Val::new(1018564082), Val::new(356040847), Val::new(6921173),
        Val::new(865613739), Val::new(1401029826), Val::new(1157587805), Val::new(1694194150),
        Val::new(1896880238), Val::new(88368571), Val::new(1349348652), Val::new(2027358192),
        Val::new(380015572), Val::new(1749008219), Val::new(245097507), Val::new(345502684)],
];
pub const POSEIDON2_W16_D5_INTERNAL_CONSTANTS: [Val; 14] = [
Val::new(1868136170), Val::new(1684664724), Val::new(983679023), Val::new(1891357693),
    Val::new(1891456615), Val::new(476121283), Val::new(1059854491), Val::new(1061508892),
    Val::new(272841724), Val::new(1160904394), Val::new(1037633668), Val::new(1955898504),
    Val::new(892602345), Val::new(2104815485)
];


// development configuration

pub const STORAGE_THRESHOLD: usize = 4;
pub const BLOWUP_FACTOR: usize = 4;

pub const NUM_NODES: usize = STORAGE_THRESHOLD * BLOWUP_FACTOR;

pub const LOG_CLUSTER_SIZE: usize = 20;
pub const LOG_FRAGMENT_SIZE: usize = 22;
pub const LOG_SEGMENT_SIZE: usize = 28;
pub const LOG_VOLUME_SIZE: usize = 37;

pub const CLUSTER_SIZE: u64 = 1 << LOG_CLUSTER_SIZE;
pub const FRAGMENT_SIZE: u64 = 1 << LOG_FRAGMENT_SIZE;
pub const SEGMENT_SIZE: u64 = 1 << LOG_SEGMENT_SIZE;
pub const VOLUME_SIZE: u64 = 1 << LOG_VOLUME_SIZE;
