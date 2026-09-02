// ── 진단 유형 ───────────────────────────────────────────────

pub const LEX: &str = "어휘 오류";
pub const SYNTAX: &str = "구문 오류";
pub const NAME: &str = "식별자 오류";
pub const MODULE: &str = "모듈 오류";
pub const PARTICLE: &str = "조사 오류";
pub const VALUE: &str = "값 오류";
pub const ARITH: &str = "산술 오류";
pub const FILE: &str = "파일 오류";
pub const STOP: &str = "종료";
pub const ERROR: &str = "오류";

// ── 진단 범주 ─────────────────────────────────────────────────

pub const HELP: &str = "도움말";
pub const NOTE: &str = "참고";

pub fn aborting(count: usize) -> String {
    format!("종료됨. (오류 {count}개)")
}

// ── 오류 경로 추적 ─────────────────────────────────────────────

pub const TRACE: &str = "오류 경로 추적:";
pub const FRAME_UNKNOWN: &str = "<알 수 없음>";
pub const FRAME_TOP: &str = "<최상단>";
pub const TRACE_OFF: &str = "`-g`로 컴파일하여 오류 경로를 추적할 수 있습니다.";

// ── 어휘 ────────────────────────────────────────────────────

pub const INDENT_ODD: &str = "들여쓰기가 잘못됨";
pub const BRACE_OPEN: &str = "'{'가 닫히지 않음";
pub const BRACE_EMPTY: &str = "'{}' 안이 비어 있음";
pub const QUOTE_OPEN: &str = "따옴표가 닫히지 않음";

pub fn bad_char(shown: &str) -> String {
    format!("사용할 수 없는 글자: {shown}")
}

pub fn not_number(raw: &str) -> String {
    format!("숫자가 아님: {raw}")
}

// ── 구문 ────────────────────────────────────────────────────

pub const NO_BLOCK: &str = "구문 블록의 들여쓰기 누락";
pub const NOT_NEGATION: &str = "'-지'에 대응하는 '않다' 누락";
pub const DECL_NOT_ONE: &str = "선언문에 지정된 표현식이 단일 값이 아님";
pub const DECL_NO_COPULA: &str = "선언문에서 '이다' 누락";
pub const HEAD_NO_QUOTATIVE: &str = "정의에서 '라는 것은' 누락";
pub const HEAD_NOT_DICT: &str = "정의 서술어 형태가 사전형이 아님";
pub const HEAD_NOT_DICT_HELP: &str = "'<구문>* <사전형>라는 것은:' 형식 준수 필요";
pub const NO_STEP_NUMBER: &str = "'<구문>간격' 형식 준수 필요";
pub const WHILE_NOT_ONE: &str = "'동안' 절의 표현식이 단일하지 않음";
pub const RETURN_NOT_ONE: &str = "반환할 값이 단일하지 않음";
pub const EXEC_CONDITIONAL: &str = "실행문에 쓸 수 없는 어미: -면";
pub const EXEC_CONDITIONAL_HELP: &str = "조건문은 '만약'으로 시작해야 함";
pub const EACH_NOT_NAME: &str = "'마다'의 수식 대상이 식별자가 아님";
pub const LOOP_NO_EACH: &str = "반복문에 '마다' 누락";
pub const LOOP_NO_RANGE: &str = "반복문 범위 누락";

pub const WANT_COLON: &str = "':'이";
pub const WANT_NEWLINE: &str = "줄바꿈이";
pub const WANT_PERIOD: &str = "'.'이";
pub const WANT_EMBEDDED: &str = "삽입된 표현식";

pub fn line_ended(what: &str) -> String {
    format!("줄바꿈시 누락됨: {what}")
}

pub fn not_wanted(what: &str, found: &str) -> String {
    format!("{what} 기대하였으나: {found}")
}

pub fn not_a_name(found: &str) -> String {
    format!("유효하지 않은 식별자: {found}")
}

pub fn not_a_particle(found: &str) -> String {
    format!("조사가 아님: {found}")
}

pub fn not_a_verb(found: &str) -> String {
    format!("동사가 아님: {found}")
}

pub fn not_a_value(found: &str) -> String {
    format!("값이 아님: {found}")
}

pub fn not_head_value(head: &str) -> String {
    format!("관형형 뒤에 `값`이 아님: '{head}'")
}

pub fn not_one(what: &str, mark: &str) -> String {
    format!("{what}{mark} 단일하지 않음")
}

pub fn not_keyword(word: &str, found: &str) -> String {
    format!("키워드 불일치: '{word}' 기대하였으나: {found}")
}

pub fn reserved_target(word: &str) -> String {
    format!("예약어에 값을 할당할 수 없음: '{word}'")
}

pub fn decl_bad_ending(ending: &str) -> String {
    format!("선언문에 허용되지 않는 어미: {ending}")
}

pub fn cond_bad_ending(ending: &str) -> String {
    format!("조건에 허용되지 않는 어미: {ending}")
}

pub fn exec_bad_ending(ending: &str) -> String {
    format!("실행문에 허용되지 않는 어미: {ending}")
}

pub fn not_thing(thing: &str) -> String {
    format!("'것'이 아님: '{thing}'")
}

pub fn noun_needs_owner(head: &str) -> String {
    format!("파생 필드 '{head}'의 수식어가 단일하지 않음")
}

pub fn head_not_phrase(found: &str) -> String {
    format!("정의는 식별자 + 조사로 시작해야 함: {found}")
}

pub fn head_twice(particle: &str) -> String {
    format!("조사 '{particle}'가 두 번 포함됨")
}

pub fn not_import_name(found: &str) -> String {
    format!("가져올 수 없는 식별자: {found}")
}

// ── 낱말 이름  ───────────────────────────────────────────────

pub const TOK_NAME: &str = "이름";
pub const TOK_VERB: &str = "동사";
pub const TOK_COPULA: &str = "'이다'";
pub const TOK_PARTICLE: &str = "조사";
pub const TOK_KEYWORD: &str = "예약어";
pub const TOK_NUMBER: &str = "수";
pub const TOK_STRING: &str = "글";
pub const TOK_SYMBOL: &str = "기호";
pub const TOK_INDENT: &str = "들여쓰기";
pub const TOK_DEDENT: &str = "내어쓰기";
pub const TOK_NEWLINE: &str = "줄 끝";
pub const TOK_EOF: &str = "파일 끝";
pub const END_AUXILIARY: &str = "보조 어미";

// ── 이름 ────────────────────────────────────────────────────

pub fn undefined(name: &str) -> String {
    format!("'{name}' 정의되지 않음")
}

pub fn verb_undefined(verb: &str) -> String {
    format!("동사 '{verb}' 정의되지 않음")
}

pub fn module_lacks(module: &str, name: &str) -> String {
    format!("모듈 '{module}'에 '{name}' 없음")
}

pub fn module_not_taken(module: &str) -> String {
    format!("모듈 '{module}' 가져오지 않음")
}

pub fn wrong_particles(verb: &str, used: &str) -> String {
    format!("'{verb}'를 조사 {used}로 호출할 수 없음")
}

pub fn similar(close: &str) -> String {
    format!("비슷한 이름: '{close}'")
}

pub fn ways(listed: &str) -> String {
    format!("조사: {listed}")
}

pub const NO_PARTICLE: &str = "없음";

// ── 모듈 ────────────────────────────────────────────────────

pub const INPUT_UNIT: &str = "<입력>";

pub fn module_cycle(module: &str) -> String {
    format!("'{module}' 순환 참조")
}

pub fn module_unreadable(module: &str, why: &str) -> String {
    format!("'{module}'을 읽을 수 없음: {why}")
}

pub fn module_missing(module: &str) -> String {
    format!("모듈 파일이 없음: {module}.sr")
}

// ── 실행 중 ─────────────────────────────────────────────────

pub const DIV_ZERO: &str = "0으로 나눌 수 없음";

pub fn arg_not_number(verb: &str, kind: &str, shown: &str) -> String {
    format!("'{verb}'의 인자가 수가 아님: {kind} {shown}")
}

pub fn no_field(field: &str) -> String {
    format!("'{field}' 접근할 수 없음")
}

pub fn no_field_on(kind: &str, field: &str) -> String {
    format!("{kind}에 '{field}' 접근할 수 없음")
}

pub fn empty_at(field: &str, size: usize) -> String {
    format!("'{field}'에 접근할 수 없음 (크기: {size})")
}

pub fn place_not_int(shown: &str) -> String {
    format!("자리가 정수가 아님: {shown}")
}

pub fn no_place(kind: &str) -> String {
    format!("{kind}에 자리가 없음")
}

pub fn out_of_range(index: i64, size: usize) -> String {
    format!("{index}번째 없음 (크기: {size})")
}

pub fn cannot_order(verb: &str, left: &str, right: &str) -> String {
    format!("'{verb}'로 비교할 수 없음: {left}, {right}")
}

pub fn cannot_convert(kind: &str, shown: &str) -> String {
    format!("{kind}로 바꿀 수 없음: {shown}")
}

pub fn unknown_kind(kind: &str) -> String {
    format!("바꿀 수 없는 자료형: '{kind}'")
}

pub fn no_return(name: &str) -> String {
    format!("'{name}' 반환하는 값이 없음")
}

pub fn not_bool(name: &str, kind: &str, shown: &str) -> String {
    format!("'{name}'의 값이 논리값이 아님: {kind} {shown}")
}

pub fn bad_mode(mode: &str) -> String {
    format!("정의되지 않은 방식: '{mode}'")
}

pub fn cannot_open(name: &str, why: &str) -> String {
    format!("'{name}'을 열 수 없음: {why}")
}

pub fn cannot_read(why: &str) -> String {
    format!("읽을 수 없음: {why}")
}

pub fn cannot_write(why: &str) -> String {
    format!("쓸 수 없음: {why}")
}

pub fn not_descriptor(verb: &str, shown: &str) -> String {
    format!("'{verb}'의 서술자가 정수가 아님: {shown}")
}

// ── 명령줄 ──────────────────────────────────────────────────

pub const USAGE: &str = "\
새롬

  saeromc <파일.sr> [-o <출력>] [-O2] [-g]
  saeromc --check <파일.sr>
  saeromc --emit-llvm <파일.sr>
";

pub fn source_unreadable(path: &str, why: &str) -> String {
    format!("파일을 읽을 수 없음: {path} ({why})")
}

pub fn write_failed(path: &str, why: &str) -> String {
    format!("{path}를 쓸 수 없음: {why}")
}

pub fn clang_missing(why: &str) -> String {
    format!("clang을 호출할 수 없음: {why}")
}

pub fn link_failed(details: &str) -> String {
    format!("링크 실패\n{details}")
}

pub fn runtime_missing(name: &str, looked: &str) -> String {
    format!("런타임 {name} 을 찾을 수 없음. ({looked})")
}
