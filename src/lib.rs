pub mod benchmarks;
pub mod prelude; // not `prelude::*`; we want to be explicit about what we import
mod prototype; // not public; will be replaced by legit
pub mod test_utils; // TODO:
                    //   Once this becomes a library, this needs to become private, but for now it is convenient
                    //   to have it accessible from outside binaries.
mod expression_components;
mod xml_parsing;

#[cfg(test)]
mod tests {
    use crate::prototype::{Expression, UpdateFn};

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
        let _sane = r#"
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
                        let expression = Expression::<u8>::try_from_xml(&mut xml);
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
                        let update_fn = UpdateFn::<u8>::try_from_xml(&mut xml);
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
