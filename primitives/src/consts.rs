use p3_mersenne_31::Mersenne31;

// Computed at ../examples/gen_seedboxes.rs

pub const POSEIDON2_M31_W16_D5_ROUNDS_F: usize = 8;
pub const POSEIDON2_M31_W16_D5_ROUNDS_P: usize = 14;
pub const POSEIDON2_M31_W16_D5_EXTERNAL_CONSTANTS: [[Mersenne31; 16]; 8] = [
    [Mersenne31::new(1428922684), Mersenne31::new(2022196109), Mersenne31::new(1224505130), Mersenne31::new(984282662),
        Mersenne31::new(1745528643), Mersenne31::new(1884925147), Mersenne31::new(1845326973), Mersenne31::new(976109012),
        Mersenne31::new(364320740), Mersenne31::new(1169816424), Mersenne31::new(1266509633), Mersenne31::new(1147500482),
        Mersenne31::new(804946803), Mersenne31::new(1336891277), Mersenne31::new(1923680287), Mersenne31::new(1051112063)],
    [Mersenne31::new(617202902), Mersenne31::new(1328322895), Mersenne31::new(809658739), Mersenne31::new(728996169),
        Mersenne31::new(367124292), Mersenne31::new(1183101044), Mersenne31::new(2017892963), Mersenne31::new(797916161),
        Mersenne31::new(1689484235), Mersenne31::new(1657723214), Mersenne31::new(1725191991), Mersenne31::new(607916694),
        Mersenne31::new(304711241), Mersenne31::new(991633463), Mersenne31::new(1341032671), Mersenne31::new(1455985172)],
    [Mersenne31::new(940327040), Mersenne31::new(1836866420), Mersenne31::new(1744330360), Mersenne31::new(1728313833),
        Mersenne31::new(1256787822), Mersenne31::new(143243872), Mersenne31::new(394906775), Mersenne31::new(93462334),
        Mersenne31::new(2095314515), Mersenne31::new(1438973973), Mersenne31::new(1925653183), Mersenne31::new(1615496024),
        Mersenne31::new(772213231), Mersenne31::new(1188568581), Mersenne31::new(411016683), Mersenne31::new(452512591)],
    [Mersenne31::new(913633223), Mersenne31::new(1119952228), Mersenne31::new(2147150098), Mersenne31::new(1631257849),
        Mersenne31::new(722026530), Mersenne31::new(51210008), Mersenne31::new(669586161), Mersenne31::new(391858424),
        Mersenne31::new(1872572836), Mersenne31::new(1530649179), Mersenne31::new(1905358042), Mersenne31::new(712337723),
        Mersenne31::new(273042458), Mersenne31::new(143817816), Mersenne31::new(2105695752), Mersenne31::new(418301610)],
    [Mersenne31::new(760850064), Mersenne31::new(724582512), Mersenne31::new(1175911295), Mersenne31::new(1686822328),
        Mersenne31::new(1838736009), Mersenne31::new(1027362987), Mersenne31::new(45299051), Mersenne31::new(326225160),
        Mersenne31::new(1722439737), Mersenne31::new(202954879), Mersenne31::new(433482402), Mersenne31::new(717784287),
        Mersenne31::new(957447280), Mersenne31::new(2072056797), Mersenne31::new(1476433164), Mersenne31::new(1961211085)],
    [Mersenne31::new(1402211604), Mersenne31::new(2047616321), Mersenne31::new(1725105359), Mersenne31::new(1403872103),
        Mersenne31::new(636199198), Mersenne31::new(711763034), Mersenne31::new(755524500), Mersenne31::new(1146269098),
        Mersenne31::new(440942860), Mersenne31::new(172467545), Mersenne31::new(1346808457), Mersenne31::new(680815102),
        Mersenne31::new(1145397703), Mersenne31::new(493957525), Mersenne31::new(1518357280), Mersenne31::new(811756323)],
    [Mersenne31::new(1599785888), Mersenne31::new(384859669), Mersenne31::new(1834738991), Mersenne31::new(349292068),
        Mersenne31::new(1562910107), Mersenne31::new(469337841), Mersenne31::new(854962023), Mersenne31::new(1219794154),
        Mersenne31::new(614870544), Mersenne31::new(533548718), Mersenne31::new(764382489), Mersenne31::new(609018108),
        Mersenne31::new(1175651676), Mersenne31::new(533401582), Mersenne31::new(208843075), Mersenne31::new(346968022)],
    [Mersenne31::new(135087855), Mersenne31::new(1018564082), Mersenne31::new(356040847), Mersenne31::new(6921173),
        Mersenne31::new(865613739), Mersenne31::new(1401029826), Mersenne31::new(1157587805), Mersenne31::new(1694194150),
        Mersenne31::new(1896880238), Mersenne31::new(88368571), Mersenne31::new(1349348652), Mersenne31::new(2027358192),
        Mersenne31::new(380015572), Mersenne31::new(1749008219), Mersenne31::new(245097507), Mersenne31::new(345502684)],
];
pub const POSEIDON2_M31_W16_D5_INTERNAL_CONSTANTS: [Mersenne31; 14] = [
Mersenne31::new(1868136170), Mersenne31::new(1684664724), Mersenne31::new(983679023), Mersenne31::new(1891357693),
    Mersenne31::new(1891456615), Mersenne31::new(476121283), Mersenne31::new(1059854491), Mersenne31::new(1061508892),
    Mersenne31::new(272841724), Mersenne31::new(1160904394), Mersenne31::new(1037633668), Mersenne31::new(1955898504),
    Mersenne31::new(892602345), Mersenne31::new(2104815485)
];
