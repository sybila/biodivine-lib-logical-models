/// A private module which stores the implementation of the traits/structures relevant for
/// symbolic encoding of logical models.
///
/// TODO:
///     In the final library, we should re-export the relevant types from this module here.
mod symbolic_domain;

pub use symbolic_domain::{
    // todo uncomment one those working
    // GenericIntegerDomain,
    // GenericStateSpaceDomain,
    SymbolicDomain,
    UnaryIntegerDomain,
};

pub fn add(x: i32, y: i32) -> i32 {
    x + y
}

// expose the prototype module
mod prototype;
pub use prototype::*;

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    pub fn test() {
        assert_eq!(5, add(2, 3));
    }

    #[test]
    pub fn test_expression() {
        // let xml = r#"<apply> // too basic
        //     <lt/>
        //     <cn type="integer">5</cn>
        //     <ci>    x    </ci>
        // </apply>
        // <apply>
        //     <eq/>
        //     <ci>x</ci>
        //     <cn type="integer">6</cn>
        // </apply>
        // "#;
        let sane = r#"
              <apply>
                <or />
                <apply>
                  <and />
                  <apply>
                    <eq />
                    <ci> p53 </ci>
                    <cn type="integer"> 0 </cn>
                  </apply>
                  <apply>
                    <eq />
                    <ci> Mdm2cyt </ci>
                    <cn type="integer"> 2 </cn>
                  </apply>
                </apply>
                <apply>
                  <and />
                  <apply>
                    <geq />
                    <ci> p53 </ci>
                    <cn type="integer"> 1 </cn>
                  </apply>
                  <apply>
                    <eq />
                    <ci> Mdm2cyt </ci>
                    <cn type="integer"> 2 </cn>
                  </apply>
                </apply>
              </apply>
        "#;
        let with_ternary = r#"
        <apply>
        <or />
        <apply>
          <and />
          <apply>
            <eq />
            <ci> p53 </ci>
            <cn type="integer"> 0 </cn>
          </apply>
          <apply>
            <eq />
            <ci> Mdm2cyt </ci>
            <cn type="integer"> 1 </cn>
          </apply>
          <apply>
            <eq />
            <ci> DNAdam </ci>
            <cn type="integer"> 0 </cn>
          </apply>
        </apply>
        <apply>
          <and />
          <apply>
            <eq />
            <ci> p53 </ci>
            <cn type="integer"> 0 </cn>
          </apply>
          <apply>
            <eq />
            <ci> Mdm2cyt </ci>
            <cn type="integer"> 2 </cn>
          </apply>
        </apply>
        <apply>
          <and />
          <apply>
            <geq />
            <ci> p53 </ci>
            <cn type="integer"> 1 </cn>
          </apply>
          <apply>
            <eq />
            <ci> Mdm2cyt </ci>
            <cn type="integer"> 2 </cn>
          </apply>
        </apply>
      </apply>
        "#;
        let mut xml = xml::reader::EventReader::new(with_ternary.as_bytes());
        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    println!("start element {:?}", name);
                    if name.local_name == "apply" {
                        println!("parsing apply");
                        let expression = super::Expression::<u8>::try_from_xml(&mut xml);
                        println!("parsed apply {:?}", expression);
                    }
                }
                Ok(xml::reader::XmlEvent::EndDocument) => {
                    println!("end of document");
                    break;
                }
                Err(err) => {
                    println!("err: {:?}", err);
                    break;
                }
                _ => {}
            }
        }
    }

    #[test]
    pub fn test_update_fn() {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open("data/dataset.sbml").expect("cannot open file");
        let file = BufReader::new(file);

        let mut xml = xml::reader::EventReader::new(file);

        let mut indent = 0;
        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    println!("{}<{:?}>", "  ".repeat(indent), name);
                    indent += 1;
                    if name.local_name == "transition" {
                        println!("parsing transition");
                        let update_fn = super::UpdateFn::<u8>::try_from_xml(&mut xml);
                        println!("update fn: {:?}", update_fn);
                        return;
                    }
                }
                Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                    indent -= 1;
                    println!("{}</{:?} />", "  ".repeat(indent), name);
                }
                Ok(xml::reader::XmlEvent::EndDocument) => {
                    println!("end of document");
                    break;
                }
                Err(err) => {
                    println!("err: {:?}", err);
                    break;
                }
                _ => {}
            }
        }
    }
}
