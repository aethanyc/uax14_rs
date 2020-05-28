mod properties;

use crate::properties::*;
use core::str::CharIndices;

#[derive(Copy, Clone, PartialEq)]
pub enum LineBreakRule {
    Normal,
    Strict,
    Loose,
}

fn get_linebreak_property_with_rule(codepoint: char, rule: LineBreakRule) -> u8 {
    let codepoint = codepoint as usize;
    if codepoint < 0x20000 {
        if rule == LineBreakRule::Strict {
            // CJ is mapped as NS on default
            return UAX14_PROPERTY_TABLE[codepoint / 1024][(codepoint & 0x3ff)];
        }
        if rule == LineBreakRule::Loose {
            let prop = match codepoint {
                0x2010 => ID,
                0x2013 => ID,
                0x3005 => ID,
                0x303B => ID,
                0x309D => ID,
                0x309E => ID,
                0x30FD => ID,
                0x30FE => ID,
                _ => UAX14_PROPERTY_TABLE[codepoint / 1024][(codepoint & 0x3ff)],
            };
            return match prop {
                CJ => ID,
                _ => prop,
            };
        }

        if rule == LineBreakRule::Normal {
            let prop = match codepoint {
                0x3005 => ID,
                0x303B => ID,
                0x309D => ID,
                0x309E => ID,
                0x30FD..=0x30FE => ID,
                _ => UAX14_PROPERTY_TABLE[codepoint / 1024][(codepoint & 0x3ff)],
            };
            return match prop {
                CJ => ID,
                _ => prop,
            };
        }

        return UAX14_PROPERTY_TABLE[codepoint / 1024][(codepoint & 0x3ff)];
    }

    match codepoint {
        0x20000..=0x2fffd => ID,
        0x30000..=0x3fffd => ID,
        0xe0001 => CM,
        0xe0020..=0xe007f => CM,
        0xe0100..=0xe01ef => CM,
        _ => XX,
    }
}

fn get_linebreak_property_utf32_with_rule(codepoint: u32, rule: LineBreakRule) -> u8 {
    get_linebreak_property_with_rule(core::char::from_u32(codepoint).unwrap(), rule)
}

fn get_linebreak_property(codepoint: char) -> u8 {
    get_linebreak_property_with_rule(codepoint, LineBreakRule::Strict)
}

fn is_break(current: u8, next: u8) -> bool {
    let rule = UAX14_RULE_TABLE[((current as usize) - 1) * PROP_COUNT + (next as usize) - 1];
    if rule == -1 {
        return false;
    }
    true
}

fn get_break_state(current: u8, next: u8) -> i8 {
    UAX14_RULE_TABLE[((current as usize) - 1) * PROP_COUNT + (next as usize) - 1]
}

pub struct LineBreakIterator<'a> {
    iter: CharIndices<'a>,
    current: Option<(usize, char)>,
    break_rule: LineBreakRule,
}

impl<'a> Iterator for LineBreakIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_eof() {
            return None;
        }

        loop {
            let mut current_prop = self.get_linebreak_property();
            // Handle LB25
            // ( PR | PO) ? ( OP | HY ) ? NU (NU | SY | IS) * (CL | CP) ? ( PR | PO) ?
            if current_prop == PR
                || current_prop == PO
                || current_prop == OP
                || current_prop == OP_EA
                || current_prop == HY
                || current_prop == NU
            {
                let backup = self.iter.clone();
                let mut current = self.current;
                let mut state = current_prop;

                let mut prev = current;
                let mut prev_state = state;

                if state == PR || state == PO {
                    current = self.iter.next();
                    if current.is_some() {
                        state =
                            get_linebreak_property_with_rule(current.unwrap().1, self.break_rule);
                    }
                    // If reaching EOF, restore iterator
                }
                if state == OP || state == HY || state == OP_EA {
                    prev = current;
                    prev_state = state;

                    current = self.iter.next();
                    if current.is_some() {
                        state = get_linebreak_property(current.unwrap().1);
                    }
                    // If reaching EOF, restore iterator
                }
                if state == NU {
                    let mut backup = self.iter.clone();
                    current_prop = state;

                    prev = current;

                    current = self.iter.next();
                    if current.is_none() {
                        // EOF
                        self.current = None;
                        return Some(prev.unwrap().0 + prev.unwrap().1.len_utf8());
                    }
                    state = get_linebreak_property_with_rule(current.unwrap().1, self.break_rule);
                    loop {
                        if state == NU || state == SY || state == IS {
                            backup = self.iter.clone();
                            current_prop = state;

                            prev = current;

                            current = self.iter.next();
                            if current.is_none() {
                                // EOF
                                self.current = None;
                                return Some(prev.unwrap().0 + prev.unwrap().1.len_utf8());
                            }
                            state = get_linebreak_property(current.unwrap().1);
                            continue;
                        }
                        break;
                    }
                    if state == CL || state == CP {
                        backup = self.iter.clone();
                        current_prop = state;

                        prev = current;

                        current = self.iter.next();
                        if current.is_some() {
                            state = get_linebreak_property(current.unwrap().1);
                        }
                        // If reaching EOF, restore iterator
                    }
                    if state == PR || state == PO {
                        self.current = current;
                        continue;
                    }

                    // Restore iterator that is NU/CL/CP position.
                    self.iter = backup;
                    self.current = prev;
                } else {
                    // Not match for LB25
                    self.iter = backup;
                }
            }

            let next = self.iter.next();
            if next.is_none() {
                // EOF
                let t = self.current.unwrap();
                self.current = None;
                return Some(t.0 + t.1.len_utf8());
            }
            self.current = next;
            let next_prop = self.get_linebreak_property();

            // Resolve state.
            let mut break_state = get_break_state(current_prop, next_prop);
            if break_state >= 0 as i8 {
                loop {
                    let prev = self.current.unwrap();
                    self.current = self.iter.next();
                    if self.current.is_none() {
                        // EOF
                        return Some(prev.0 + prev.1.len_utf8());
                    }

                    let prop = self.get_linebreak_property();
                    break_state = get_break_state(break_state as u8, prop);
                    if break_state < 0 {
                        break;
                    }
                }
                if break_state == -1 {
                    continue;
                }
                return Some(self.current.unwrap().0);
            }
            if is_break(current_prop, next_prop) {
                return Some(self.current.unwrap().0);
            }
        }
    }
}

impl<'a> LineBreakIterator<'a> {
    pub fn new(input: &str) -> LineBreakIterator {
        LineBreakIterator {
            iter: input.char_indices(),
            current: None,
            break_rule: LineBreakRule::Strict,
        }
    }

    pub fn new_with_break_rule(input: &str, break_rule: LineBreakRule) -> LineBreakIterator {
        LineBreakIterator {
            iter: input.char_indices(),
            current: None,
            break_rule: break_rule,
        }
    }

    fn get_linebreak_property(&mut self) -> u8 {
        get_linebreak_property_with_rule(self.current.unwrap().1, self.break_rule)
    }

    fn iterator_next(&mut self) {
        self.current = self.iter.next();
    }

    #[inline]
    fn is_eof(&mut self) -> bool {
        if self.current.is_none() {
            self.current = self.iter.next();
            if self.current.is_none() {
                return true;
            }
        }
        return false;
    }
}

macro_rules! iterator_impl {
    ($name:ident, $attr:ty) => {
        pub struct $name<'a> {
            iter: &'a [$attr],
            current: usize,
            break_rule: LineBreakRule,
        }

        impl<'a> Iterator for $name<'a> {
            type Item = usize;

            fn next(&mut self) -> Option<Self::Item> {
                if self.is_eof() {
                    return None;
                }

                let mut current_prop = self.get_linebreak_property();
                loop {
                    if self.process_lb25(current_prop) {
                        current_prop = self.get_linebreak_property();
                        // LB25 is processed, but iter isn't updated when one NU only.
                        if current_prop != NU {
                            continue;
                        }
                    }

                    // Fetch next char
                    if self.iterator_next().is_none() {
                        return Some(self.last());
                    }

                    let next_prop = self.get_linebreak_property();

                    // Resolve state.
                    let mut break_state = get_break_state(current_prop, next_prop);
                    if break_state >= 0 {
                        loop {
                            if self.iterator_next().is_none() {
                                return Some(self.last());
                            }
                            let prop = self.get_linebreak_property();
                            break_state = get_break_state(break_state as u8, prop);
                            if break_state < 0 {
                                break;
                            }
                        }
                        if break_state == -1 {
                            current_prop = self.get_linebreak_property();
                            continue;
                        }
                        return Some(self.current);
                    }

                    if is_break(current_prop, next_prop) {
                        return Some(self.current);
                    }
                    current_prop = next_prop;
                }
                return Some(self.current);
            }
        }

        impl<'a> $name<'a> {
            pub fn new(input: &[$attr]) -> $name {
                $name {
                    iter: input,
                    current: 0,
                    break_rule: LineBreakRule::Strict,
                }
            }

            pub fn new_with_break_rule(input: &[$attr], break_rule: LineBreakRule) -> $name {
                $name {
                    iter: input,
                    current: 0,
                    break_rule: break_rule,
                }
            }

            // processing LB25 rule. This isn't resolved by state machine table.
            // return false if not handled or reached to EOF.
            // ( PR | PO) ? ( OP | HY ) ? NU (NU | SY | IS) * (CL | CP) ? ( PR | PO) ?
            fn process_lb25(&mut self, current_prop: u8) -> bool {
                if current_prop != PR
                    && current_prop != PO
                    && current_prop != OP
                    && current_prop != OP_EA
                    && current_prop != HY
                    && current_prop != NU
                {
                    return false;
                }

                let start_marker = self.save_iterator();
                let mut state = current_prop;
                if state == PR || state == PO {
                    self.iterator_next();
                    if !self.is_eof() {
                        state = self.get_linebreak_property();
                    }
                }
                if state == OP || state == HY || state == OP_EA {
                    self.iterator_next();
                    if !self.is_eof() {
                        state = self.get_linebreak_property();
                    }
                }
                if state != NU {
                    // Not match for LB25
                    self.current = start_marker;
                    return false;
                }

                let mut prev = self.save_iterator();
                if self.iterator_next().is_none() {
                    //self.restore_iterator(prev);
                    return false;
                }

                loop {
                    state = self.get_linebreak_property();
                    if state != NU && state != SY && state != IS {
                        break;
                    }
                    prev = self.save_iterator();
                    if self.iterator_next().is_none() {
                        //self.restore_iterator(prev);
                        return false;
                    }
                }
                if state == CL || state == CP {
                    prev = self.save_iterator();
                    if self.iterator_next().is_none() {
                        //self.restore_iterator(prev);
                        return false;
                    }
                    state = self.get_linebreak_property();
                }
                if state == PR || state == PO {
                    return true;
                }
                self.restore_iterator(prev);
                return true;
            }

            fn get_linebreak_property(&mut self) -> u8 {
                let mut current = self.iter[self.current] as u32;
                if (current & 0xfc00) == 0xd800 {
                    if self.current + 1 < self.iter.len() {
                        let next = self.iter[self.current + 1] as u32;
                        if (next & 0xfc00) == 0xdc00 {
                            current = ((current & 0x3ff) << 10) + (next & 0x3ff);
                        }
                    }
                }
                get_linebreak_property_utf32_with_rule(current as u32, self.break_rule)
            }

            #[inline]
            fn iterator_next(&mut self) -> Option<$attr> {
                if self.is_eof() {
                    return None;
                }
                self.current = self.current + 1;
                let prev = self.iter[self.current - 1] as u32;
                if (prev & 0xfc00) == 0xd800
                    && ((self.iter[self.current] as u32) & 0xfc00) == 0xdc00
                {
                    self.current = self.current + 1;
                }
                if self.is_eof() {
                    return None;
                }
                Some(self.iter[self.current])
            }

            #[inline]
            fn save_iterator(&mut self) -> usize {
                self.current
            }

            #[inline]
            fn restore_iterator(&mut self, position: usize) {
                self.current = position;
            }

            #[inline]
            fn last(&mut self) -> usize {
                self.iter.len()
            }

            #[inline]
            fn is_eof(&self) -> bool {
                self.current >= self.iter.len()
            }
        }
    };
}

iterator_impl!(LineBreakIteratorLatin1, u8);
iterator_impl!(LineBreakIteratorUTF16, u16);

#[cfg(test)]
mod tests {
    use crate::get_linebreak_property;
    use crate::is_break;
    use crate::properties::*;
    use crate::LineBreakIterator;
    use crate::LineBreakIteratorLatin1;
    use crate::LineBreakIteratorUTF16;
    use crate::LineBreakRule;

    #[test]
    fn linebreak_propery() {
        assert_eq!(get_linebreak_property('\u{0020}'), SP);
        assert_eq!(get_linebreak_property('\u{0022}'), QU);
        assert_eq!(get_linebreak_property('('), OP);
        assert_eq!(get_linebreak_property('\u{0030}'), NU);
        assert_eq!(get_linebreak_property('['), OP);
        assert_eq!(get_linebreak_property('\u{1f3fb}'), EM);
        assert_eq!(get_linebreak_property('\u{20000}'), ID);
        assert_eq!(get_linebreak_property('\u{e0020}'), CM);
        assert_eq!(get_linebreak_property('\u{3041}'), CJ);
        assert_eq!(get_linebreak_property('\u{0025}'), PO);
        assert_eq!(get_linebreak_property('\u{00A7}'), AI);
        assert_eq!(get_linebreak_property('\u{50005}'), XX);
        assert_eq!(get_linebreak_property('\u{17D6}'), NS);
        assert_eq!(get_linebreak_property('\u{2014}'), B2);
    }

    #[test]
    fn break_rule() {
        // LB4
        assert_eq!(is_break(BK, AL), true);
        // LB5
        assert_eq!(is_break(CR, LF), false);
        assert_eq!(is_break(CR, AL), true);
        assert_eq!(is_break(LF, AL), true);
        assert_eq!(is_break(NL, AL), true);
        // LB6
        assert_eq!(is_break(AL, BK), false);
        assert_eq!(is_break(AL, CR), false);
        assert_eq!(is_break(AL, LF), false);
        assert_eq!(is_break(AL, NL), false);
        // LB7
        assert_eq!(is_break(AL, SP), false);
        assert_eq!(is_break(AL, ZW), false);
        // LB8
        // LB8a
        assert_eq!(is_break(ZWJ, AL), false);
        // LB11
        assert_eq!(is_break(AL, WJ), false);
        assert_eq!(is_break(WJ, AL), false);
        // LB12
        assert_eq!(is_break(GL, AL), false);
        // LB12a
        assert_eq!(is_break(AL, GL), false);
        assert_eq!(is_break(SP, GL), true);
        // LB13
        assert_eq!(is_break(AL, CL), false);
        assert_eq!(is_break(AL, CP), false);
        assert_eq!(is_break(AL, EX), false);
        assert_eq!(is_break(AL, IS), false);
        assert_eq!(is_break(AL, SY), false);
        // LB18
        assert_eq!(is_break(SP, AL), true);
        // LB19
        assert_eq!(is_break(AL, QU), false);
        assert_eq!(is_break(QU, AL), false);
        // LB20
        assert_eq!(is_break(AL, CB), true);
        assert_eq!(is_break(CB, AL), true);
        // LB20
        assert_eq!(is_break(AL, BA), false);
        assert_eq!(is_break(AL, HY), false);
        assert_eq!(is_break(AL, NS), false);
        assert_eq!(is_break(BB, AL), false);
        // LB21
        assert_eq!(is_break(AL, BA), false);
        // LB21a
        // LB21b
        assert_eq!(is_break(SY, HL), false);
        // LB22
        assert_eq!(is_break(AL, IN), false);
        // LB 23
        assert_eq!(is_break(AL, NU), false);
        assert_eq!(is_break(HL, NU), false);
        // LB 23a
        assert_eq!(is_break(PR, ID), false);
        // LB26
        assert_eq!(is_break(JL, JL), false);
        assert_eq!(is_break(JL, JV), false);
        assert_eq!(is_break(JL, H2), false);
        // LB27
        assert_eq!(is_break(JL, IN), false);
        assert_eq!(is_break(JL, PO), false);
        assert_eq!(is_break(PR, JL), false);
        // LB28
        assert_eq!(is_break(AL, AL), false);
        assert_eq!(is_break(HL, AL), false);
        // LB29
        assert_eq!(is_break(IS, AL), false);
        assert_eq!(is_break(IS, HL), false);
        // LB30b
        assert_eq!(is_break(EB, EM), false);
        // LB31
        assert_eq!(is_break(ID, ID), true);
    }

    #[test]
    fn linebreak() {
        let mut iter = LineBreakIterator::new("hello world");
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(11), iter.next());
        assert_eq!(None, iter.next());

        iter = LineBreakIterator::new("$10 $10");
        assert_eq!(Some(4), iter.next());
        assert_eq!(Some(7), iter.next());

        // LB10

        // LB14
        iter = LineBreakIterator::new("[  abc def");
        assert_eq!(Some(7), iter.next());
        assert_eq!(Some(10), iter.next());
        assert_eq!(None, iter.next());

        let input: [u8; 10] = [0x5B, 0x20, 0x20, 0x61, 0x62, 0x63, 0x20, 0x64, 0x65, 0x66];
        let mut iter_u8 = LineBreakIteratorLatin1::new(&input);
        assert_eq!(Some(7), iter_u8.next());
        assert_eq!(Some(10), iter_u8.next());
        assert_eq!(None, iter_u8.next());

        let input: [u16; 10] = [0x5B, 0x20, 0x20, 0x61, 0x62, 0x63, 0x20, 0x64, 0x65, 0x66];
        let mut iter_u16 = LineBreakIteratorUTF16::new(&input);
        assert_eq!(Some(7), iter_u16.next());

        // LB15
        iter = LineBreakIterator::new("abc\u{0022}  (def");
        assert_eq!(Some(10), iter.next());

        let input: [u8; 10] = [0x61, 0x62, 0x63, 0x22, 0x20, 0x20, 0x28, 0x64, 0x65, 0x66];
        let mut iter_u8 = LineBreakIteratorLatin1::new(&input);
        assert_eq!(Some(10), iter_u8.next());

        let input: [u16; 10] = [0x61, 0x62, 0x63, 0x22, 0x20, 0x20, 0x28, 0x64, 0x65, 0x66];
        let mut iter_u16 = LineBreakIteratorUTF16::new(&input);
        assert_eq!(Some(10), iter_u16.next());

        // LB16
        iter = LineBreakIterator::new("\u{0029}\u{203C}");
        assert_eq!(Some(4), iter.next());
        iter = LineBreakIterator::new("\u{0029}  \u{203C}");
        assert_eq!(Some(6), iter.next());

        let input: [u16; 4] = [0x29, 0x20, 0x20, 0x203c];
        let mut iter_u16 = LineBreakIteratorUTF16::new(&input);
        assert_eq!(Some(4), iter_u16.next());

        // LB17
        iter = LineBreakIterator::new("\u{2014}\u{2014}aa");
        assert_eq!(Some(6), iter.next());
        iter = LineBreakIterator::new("\u{2014}  \u{2014}aa");
        assert_eq!(Some(8), iter.next());

        iter = LineBreakIterator::new("\u{2014}\u{2014}  \u{2014}\u{2014}123 abc");
        assert_eq!(Some(14), iter.next());
        assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());

        // LB25
        let mut iter = LineBreakIterator::new("(0,1)+(2,3)");
        assert_eq!(Some(11), iter.next());
        let input: [u16; 11] = [
            0x28, 0x30, 0x2C, 0x31, 0x29, 0x2B, 0x28, 0x32, 0x2C, 0x33, 0x29,
        ];
        let mut iter_u16 = LineBreakIteratorUTF16::new(&input);
        assert_eq!(Some(11), iter_u16.next());

        let input: [u16; 13] = [
            0x2014, 0x2014, 0x20, 0x20, 0x2014, 0x2014, 0x31, 0x32, 0x33, 0x20, 0x61, 0x62, 0x63,
        ];
        let mut iter_u16 = LineBreakIteratorUTF16::new(&input);
        assert_eq!(Some(6), iter_u16.next());

        iter = LineBreakIterator::new("\u{1F3FB} \u{1F3FB}");
        assert_eq!(Some(5), iter.next());
    }

    #[test]
    fn linebreak_strict() {
        let mut iter = LineBreakIterator::new_with_break_rule(
            "サンプル文\u{3041}サンプル文",
            LineBreakRule::Strict,
        );
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());
        assert_eq!(Some(24), iter.next());
        assert_eq!(Some(27), iter.next());

        // from css/css-text/line-break/line-break-loose-012.xht
        let mut iter =
            LineBreakIterator::new_with_break_rule("サンプル文\u{30FC}サ", LineBreakRule::Strict);
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());

        // from css/css-text/line-break/line-break-loose-018.xht
        let mut iter = LineBreakIterator::new_with_break_rule(
            "サンプル文\u{20ac}サンプル文",
            LineBreakRule::Strict,
        );
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(15), iter.next());
        //assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());
        assert_eq!(Some(24), iter.next());
        assert_eq!(Some(27), iter.next());
    }

    #[test]
    fn linebreak_loose() {
        // from css/css-text/line-break/line-break-loose-011.xht
        let mut iter =
            LineBreakIterator::new_with_break_rule("サンプル文\u{3041}サ", LineBreakRule::Loose);
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(15), iter.next());
        assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());

        // from css/css-text/line-break/line-break-loose-012.xht
        let mut iter =
            LineBreakIterator::new_with_break_rule("サンプル文\u{30FC}サ", LineBreakRule::Loose);
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(15), iter.next());
        assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());

        // from css/css-text/line-break/line-break-loose-013.xht
        let mut iter =
            LineBreakIterator::new_with_break_rule("サンプル文\u{301C}サ", LineBreakRule::Loose);
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        //assert_eq!(Some(15), iter.next());
        assert_eq!(Some(18), iter.next());
        assert_eq!(Some(21), iter.next());

        // from css/css-text/line-break/line-break-loose-018.xht
        let mut iter =
            LineBreakIterator::new_with_break_rule("サンプル文\u{20ac}サ", LineBreakRule::Loose);
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(6), iter.next());
        assert_eq!(Some(9), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(15), iter.next());
        assert_eq!(Some(21), iter.next());
    }
}
