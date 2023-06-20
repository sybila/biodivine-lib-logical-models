use biodivine_lib_bdd as bdd;
use bdd::*;
use bdd::boolean_expression::BooleanExpression;

pub fn foo() {
    println!("foo in prototype/sol.rs");
}

pub fn tutorial() {
    println!("bdd tutorial");
    // bdd::bdd!("");s

    let mut var_builder = bdd::BddVariableSetBuilder::new();
    let [a, b, c] = var_builder.make(&["a", "b", "c"]);
    let vars = var_builder.build();

    let x = vars.eval_expression_string("(a <=> !b) | c ^ a");
    let y = bdd::bdd!(vars, (a <=> (!b)) | (c ^ a));
    let z = vars.mk_literal(a, true).iff(&vars.mk_literal(b, false)).or(&vars.mk_literal(c, true).xor(&vars.mk_literal(a, true)));

    assert!(!x.is_false());
    assert_eq!(6.0, x.cardinality());
    assert_eq!(x, y);
    assert_eq!(y, z);
    assert_eq!(z, x);

    for valuation in x.sat_valuations() {
        assert!(x.eval_in(&valuation));
    }

    let anonymous = bdd::BddVariableSet::new_anonymous(3);
    println!("anonymous vars: {:?}", anonymous.variables());
    println!("var 0: {:?}", anonymous.variables()[0]);
    println!("its name: {}", anonymous.name_of(anonymous.variables()[0]));

    let true_bdd = anonymous.mk_true();
    println!("true bdd: {:?}", true_bdd);

    let var0 = anonymous.mk_var(anonymous.variables()[0]);
    println!("var 0: {:?}", var0);

    let not_var0 = anonymous.mk_not_var(anonymous.variables()[0]);
    println!("not var 0: {:?}", not_var0);

    // bdd manipulation
    let vars = BddVariableSet::new(&["a", "b", "c"]);
    let a = vars.mk_var_by_name("a");
    let b = vars.mk_var_by_name("b");
    let c = vars.mk_var_by_name("c");

    let a_and_b = a.and(&b);
    let b_or_c = b.or(&c);
    let _a_and_b_not_eq_b_or_c = a_and_b.iff(&b_or_c).not();

    let variables = BddVariableSet::new(&["a", "b", "c"]);

    let f1 = variables.eval_expression_string("a & (!b => c ^ a)");

    let expression = BooleanExpression::try_from("(b | a ^ c) & a").unwrap();
    let f2 = variables.eval_expression(&expression);

    let _f3 = variables.safe_eval_expression(&expression).unwrap(); // nice we can catch errors c:

    assert_eq!(f1, f2);

    // ah okay this made me realize how the array representation works
    let vars = BddVariableSet::new_anonymous(9);
    let true_bdd = vars.mk_true();
    println!("true bdd: {:?}", true_bdd);
    let false_bdd = vars.mk_false();
    println!("false bdd: {:?}", false_bdd);


    // serialization
    let vars = BddVariableSet::new(&["a", "b", "c"]);
    let complex = vars.eval_expression_string("a & (!b => c ^ a)");
    println!("complex: {:?}", complex);
    println!("complex in .dot format: {}", complex.to_dot_string(&vars, false));

    complex.sat_valuations().for_each(|valuation| {
        println!("valuation: {:?}", valuation);
        assert!(complex.eval_in(&valuation));
    });


    // advanced operations
    let vars = BddVariableSet::new_anonymous(5);
    let variables = vars.variables();
    let bdd = vars.eval_expression_string("(x_0 & !x_1) | (!x_0 & x_3)");

    let select_x0_true = bdd.var_select(variables[0], true);
    let select_x0_false = bdd.var_select(variables[0], false);

    assert_eq!(vars.eval_expression_string("x_0 & !x_1"), select_x0_true);
    assert_eq!(vars.eval_expression_string("!x_0 & x_3"), select_x0_false);

    println!("assertions ok");
}