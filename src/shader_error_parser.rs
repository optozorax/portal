pub fn shader_error_parser(error: &str) -> Vec<Result<(usize, &str), &str>> {
    fn expect_str(input: &mut &str, to_expect: &str) -> Option<()> {
        if to_expect.chars().count() > input.chars().count() {
            return None;
        }

        if input.chars().zip(to_expect.chars()).any(|(a, b)| a != b) {
            return None;
        }

        *input = &input[to_expect.len()..];
        Some(())
    }

    fn expect_int(input: &mut &str) -> Option<usize> {
        let pos = input
            .char_indices()
            .take_while(|(_, c)| c.is_digit(10))
            .last()
            .map(|(i, c)| i + c.len_utf8())?;
        let lineno: usize = input[..pos].parse().ok()?;
        *input = &input[pos..];
        Some(lineno)
    }

    // Try parse format `0(270) : error C0000: syntax error, unexpected '}' at token "}"`
    // This format is noticed on native Linux
    fn try_parse_1(mut line: &str) -> Option<(usize, &str)> {
        expect_str(&mut line, "0(")?;
        let lineno = expect_int(&mut line)?;
        expect_str(&mut line, ") : error ")?;
        Some((lineno, line))
    }

    fn try_parse_2(mut line: &str) -> Option<(usize, &str)> {
        expect_str(&mut line, "0(")?;
        let lineno = expect_int(&mut line)?;
        expect_str(&mut line, ") : warning ")?;
        Some((lineno, line))
    }

    // Try parse format `ERROR: 0:586: 'pos' : redefinition`
    // This format is noticed on Firefox + Linux
    fn try_parse_3(mut line: &str) -> Option<(usize, &str)> {
        expect_str(&mut line, "ERROR: 0:")?;
        let lineno = expect_int(&mut line)?;
        expect_str(&mut line, ": ")?;
        Some((lineno, line))
    }

    error
        .split('\n')
        .map(|line| {
            if let Some(r) = try_parse_1(line) {
                Ok(r)
            } else if let Some(r) = try_parse_2(line) {
                Ok(r)
            } else if let Some(r) = try_parse_3(line) {
                Ok(r)
            } else {
                macroquad::prelude::miniquad::error!("can't parse line: `{}`", line);
                Err(line)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_error_parser_test() {
        let linux1 = r#"0(270) : error C0000: syntax error, unexpected '}' at token "}"
0(286) : error C1503: undefined variable "a"
0(286) : error C1503: undefined variable "n"
0(287) : error C1503: undefined variable "b"
0(287) : error C1503: undefined variable "n"
0(288) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(327) : error C1503: undefined variable "two_lines_nearest_points"
0(327) : error C1503: undefined variable "l"
0(327) : error C1503: undefined variable "r"
0(329) : error C1503: undefined variable "l"
0(329) : error C1503: undefined variable "l"
0(330) : error C1503: undefined variable "r"
0(330) : error C1503: undefined variable "r"
0(332) : error C1059: non constant expression in initialization
0(334) : error C0000: syntax error, unexpected reserved word "if" at token "if"
0(347) : error C1503: undefined variable "u"
0(348) : error C1503: undefined variable "u"
0(349) : error C1503: undefined variable "u"
0(350) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(359) : error C1503: undefined variable "u"
0(359) : error C1038: declaration of "b" conflicts with previous declaration at 0(347)
0(360) : error C1503: undefined variable "u"
0(360) : error C1038: declaration of "c" conflicts with previous declaration at 0(348)
0(361) : error C1503: undefined variable "u"
0(361) : error C1038: declaration of "d" conflicts with previous declaration at 0(349)
0(362) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(373) : error C0000: syntax error, unexpected '}' at token "}"
0(375) : error C0000: syntax error, unexpected '(', expecting "::" at token "("
0(378) : error C1503: undefined variable "mobius_step"
0(378) : error C1503: undefined variable "r"
0(379) : error C0000: syntax error, unexpected reserved word "for" at token "for"
0(433) : error C1503: undefined variable "op"
0(433) : error C1503: undefined variable "r"
0(433) : error C1038: declaration of "b" conflicts with previous declaration at 0(347)
0(434) : error C1503: undefined variable "op"
0(434) : error C1503: undefined variable "op"
0(435) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(550) : error C1503: undefined variable "is_inside_triangle"
0(555) : error C1503: undefined variable "is_inside_triangle"
0(631) : error C1503: undefined variable "process_plane_intersection"
0(635) : error C1503: undefined variable "process_plane_intersection"
0(639) : error C1503: undefined variable "process_plane_intersection"
0(643) : error C1503: undefined variable "process_plane_intersection"
0(647) : error C1503: undefined variable "process_plane_intersection"
0(651) : error C1503: undefined variable "process_plane_intersection"
0(655) : error C1503: undefined variable "process_plane_intersection"
0(659) : error C1503: undefined variable "process_plane_intersection"
0(664) : error C1503: undefined variable "process_portal_intersection"
0(668) : error C1503: undefined variable "process_portal_intersection"
0(673) : error C1503: undefined variable "process_portal_intersection"
0(677) : error C1503: undefined variable "process_portal_intersection"
0(680) : error C1503: undefined variable "a2_mat"
0(682) : error C1503: undefined variable "process_portal_intersection"
0(686) : error C1503: undefined variable "process_portal_intersection""#;
        assert!(shader_error_parser(linux1).iter().all(|x| x.is_ok()));
        let linux2 = r#"0(277) : error C1503: undefined variable "borer_m"
0(292) : error C0000: syntax error, unexpected '}', expecting ',' or ';' at token "}"
0(284) : error C1110: function "two_lines_nearest_points" has no return statement
0(295) : error C1115: unable to find compatible overloaded function "dot(mat3, vec3)"
0(299) : error C1102: incompatible type for parameter #1 ("a.84")"#;
        assert!(shader_error_parser(linux2).iter().all(|x| x.is_ok()));
        let linux3 = r#"0(365) : warning C7022: unrecognized profile specifier "a""#;
        assert!(shader_error_parser(linux3).iter().all(|x| x.is_ok()));
        let web_linux = r#"ERROR: 0:565: 'pos' : redefinition
ERROR: 0:586: 'pos' : redefinition
ERROR: 0:606: 'pos' : redefinition
ERROR: 0:607: '<' : comparison operator only defined for scalars
ERROR: 0:607: '<' : wrong operand types - no operation '<' exists that takes a left-hand operand of type 'in highp 4-component vector of float' and a right operand of type 'const float' (or there is no acceptable conversion)
ERROR: 0:613: '<' : comparison operator only defined for scalars
ERROR: 0:613: '<' : wrong operand types - no operation '<' exists that takes a left-hand operand of type 'in highp 4-component vector of float' and a right operand of type 'const float' (or there is no acceptable conversion)"#;
        assert!(shader_error_parser(web_linux).iter().all(|x| x.is_ok()));
    }
}
