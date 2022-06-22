use chrono::{DateTime, Local};
pub fn sunset(lat: f64, lon: f64, offset: f64) -> DateTime<Local> {
}
fn _do(lat: f64, lon: f64, offset: f64) ->  {
    let today = Local::today();

    let d2 = 0.0; // d2 is the current date, # of days since Jan 1 1900?
    let f2 = d2 + 2415018.5 - offset / 24.0;
    let g2 = (f2 - 2451545.0) / 36525.0;
    let i2 = (280.46646 + g2 * (36000.76983 + g2 * 0.0003032)) % 360.0;
    let j2 = 357.52911 + g2 * (35999.05029 - 0.0001537 * g2);
    let k2 = 0.016708634 - g2 * (0.000042037 + 0.0000001267 * g2);
    let l2 = f64::sin(radians(j2)) * (1.914602 - g2 * (0.004817 + 0.000014 * g2))
        + f64::sin(radians(2.0 * j2)) * (0.019993 - 0.000101 * g2)
        + f64::sin(radians(3.0 * j2)) * 0.000289;
    let m2 = i2 + l2;
    let n2 = j2 + l2;
    let o2 = (1.000001018 * (1.0 - k2 * k2)) / (1.0 + k2 * f64::cos(radians(n2)));
    let p2 = m2 - 0.00569 - 0.00478 * f64::sin(radians(125.04 - 1934.136 * g2));
    let q2 =
        23.0 + (26.0 + (21.448 - g2 * (46.815 + g2 * (0.00059 - g2 * 0.001813))) / 60.0) / 60.0;
    let r2 = q2 + 0.00256 * f64::cos(radians(125.04 - 1934.136 * g2));
    // TODO is atan2 arg order correct?
    let s2 = degrees(f64::atan2(
        f64::cos(radians(p2)),
        f64::cos(radians(r2)) * f64::sin(radians(p2)),
    ));
    let t2 = degrees(f64::asin(f64::sin(radians(r2)) * f64::sin(radians(p2))));
    let u2 = f64::tan(radians(r2 / 2.0)) * f64::tan(radians(r2 / 2.0));
    let v2 = 4.0
        * degrees(
            u2 * f64::sin(2.0 * radians(i2)) - 2.0 * k2 * f64::sin(radians(j2))
                + 4.0 * k2 * u2 * f64::sin(radians(j2)) * f64::cos(2.0 * radians(i2))
                - 0.5 * u2 * u2 * f64::sin(4.0 * radians(i2))
                - 1.25 * k2 * k2 * f64::sin(2.0 * radians(j2)),
        );
    let w2 = degrees(f64::acos(
        f64::cos(radians(90.833)) / (f64::cos(radians(lat)) * f64::cos(radians(t2)))
            - f64::tan(radians(lat)) * f64::tan(radians(t2)),
    ));
    let x2 = (720.0 - 4.0 * lon - v2 + offset * 60.0) / 1440.0;
    let sunrise = x2 - w2 * 4.0 / 1440.0;
    let sunset = x2 + w2 * 4.0 / 1440.0;
}

fn radians(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}
fn degrees(rad: f64) -> f64 {
    rad / std::f64::consts::PI * 180.0
}

//f2 = D2+2415018.5-$B$5/24
//g2 = (F2-2451545)/36525
//i2 = MOD(280.46646+G2*(36000.76983+G2*0.0003032),360)
//j2 = 357.52911+G2*(35999.05029-0.0001537*G2)
//k2 = 0.016708634-G2*(0.000042037+0.0000001267*G2)
//l2 = SIN(RADIANS(J2))*(1.914602-G2*(0.004817+0.000014*G2))+SIN(RADIANS(2*J2))*(0.019993-0.000101*G2)+SIN(RADIANS(3*J2))*0.000289
//m2 = I2+L2
//n2 = J2+L2
//o2 = (1.000001018*(1-K2*K2))/(1+K2*COS(RADIANS(N2)))
//p2 = M2-0.00569-0.00478*SIN(RADIANS(125.04-1934.136*G2))
//q2 = 23+(26+((21.448-G2*(46.815+G2*(0.00059-G2*0.001813))))/60)/60
//r2 = Q2+0.00256*COS(RADIANS(125.04-1934.136*G2))
//s2 = DEGREES(ATAN2(COS(RADIANS(P2)),COS(RADIANS(R2))*SIN(RADIANS(P2))))
//t2 = DEGREES(ASIN(SIN(RADIANS(R2))*SIN(RADIANS(P2))))
//u2 = TAN(RADIANS(R2/2))*TAN(RADIANS(R2/2))
//v2 = 4*DEGREES(U2*SIN(2*RADIANS(I2))-2*K2*SIN(RADIANS(J2))+4*K2*U2*SIN(RADIANS(J2))*COS(2*RADIANS(I2))-0.5*U2*U2*SIN(4*RADIANS(I2))-1.25*K2*K2*SIN(2*RADIANS(J2)))
//w2 = DEGREES(ACOS(COS(RADIANS(90.833))/(COS(RADIANS(lat))*COS(RADIANS(T2)))-TAN(RADIANS(lat))*TAN(RADIANS(T2))))
//x2 =(720-4*lon-V2+offset_east*60)/1440
//y2 = X2-W2*4/1440 // sunrize
//z2 = x2 + w2*4/1440 //sunset
