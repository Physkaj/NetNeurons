use std::{fmt::Display, iter::{self, empty}};

use crate::nnetwork::{CalcNode, NodeShape};

use crate::nnetwork::Parameters;

use super::Layer;

pub struct LinearLayer {
    _w: CalcNode,
    _b: Option<CalcNode>,
    _label: String,
}

impl LinearLayer {
    pub fn from_rand(n_rows: usize, n_cols: usize, biased: bool, label: &str) -> LinearLayer {
        LinearLayer {
            _w: CalcNode::rand_from_shape((n_rows, n_cols)),
            _b: if biased {
                Some(CalcNode::rand_from_shape((n_rows, 1)))
            } else {
                None
            },
            _label: label.to_string(),
        }
    }
    pub fn from_nodes(w: CalcNode, b: Option<CalcNode>, label: &str) -> LinearLayer {
        assert!(
            !w.is_empty() && (b.is_none() || !b.as_ref().unwrap().is_empty()),
            "Cannot create layer from empty tensor."
        );
        if let Some(b) = &b {
            assert_eq!(
                w.shape().0,
                b.shape().0,
                "Bias vector must have equal number of rows as weight matrix."
            );
        }
        LinearLayer {
            _w: w,
            _b: b,
            _label: label.to_string(),
        }
    }
}

impl Display for LinearLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LinearLayer ({}): [weights: {}", self._label, self._w)?;
        if self._b.is_some() {
            write!(f, ", biases: {}", self._w)?;
        }
        writeln!(f, "]")
    }
}

impl Parameters for LinearLayer {
    fn param_iter(&self) -> Box<dyn Iterator<Item = &CalcNode> + '_> {
        let parameters = iter::once(&self._w);
        if self._b.is_some() {
            Box::new(parameters.chain(iter::once(self._b.as_ref().unwrap())))
        } else {
            Box::new(parameters)
        }
    }

    fn param_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut CalcNode> + '_> {
        let parameters = iter::once(&mut self._w);
        if self._b.is_some() {
            Box::new(parameters.chain(iter::once(self._b.as_mut().unwrap())))
        } else {
            Box::new(parameters)
        }
    }
}

impl Layer for LinearLayer {
    fn forward(&self, prev: &CalcNode) -> CalcNode {
        if self._b.is_some() {
            &self._w * prev + self._b.as_ref().unwrap()
        } else {
            &self._w * prev
        }
    }

    fn layer_name(&self) -> &str {
        &self._label
    }
}

pub struct ReshapeLayer {
    _shape: NodeShape,
    _label: String,
}

impl ReshapeLayer {
    pub fn new(shape: NodeShape, label: &str) -> Self {
        ReshapeLayer {
            _shape: shape,
            _label: label.to_string(),
        }
    }
}

impl Display for ReshapeLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ReshapeLayer ({}): [{:?}]", self._label, self._shape)
    }
}

impl Parameters for ReshapeLayer {
    fn param_iter(&self) -> Box<dyn Iterator<Item = &CalcNode> + '_> {
        Box::new(empty())
        
    }

    fn param_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut CalcNode> + '_> {
        Box::new(empty())
    }
}

impl Layer for ReshapeLayer {
    fn forward(&self, inp: &CalcNode) -> CalcNode {
        let mut out = inp.clone();
        out.reshape(self._shape);
        out
    }

    fn layer_name(&self) -> &str {
        &self._label
    }
}

#[derive(Clone)]
pub struct FunctionLayer {
    _func: &'static dyn Fn(&CalcNode) -> CalcNode,
    _formula: String,
    _label: String,
}

impl FunctionLayer {
    pub fn new(
        f: &'static dyn Fn(&CalcNode) -> CalcNode,
        formula: &str,
        label: &str,
    ) -> FunctionLayer {
        FunctionLayer {
            _func: f,
            _formula: formula.into(),
            _label: label.into(),
        }
    }

    pub fn sigmoid(inp: &CalcNode) -> CalcNode {
        (CalcNode::filled_from_shape(inp.shape(), vec![1.; inp.len()]) + (-inp).exp())
            .pow(&CalcNode::new_scalar(-1.))
    }

    pub fn tanh(inp: &CalcNode) -> CalcNode {
        let one = CalcNode::new_scalar(1.);
        let a = -inp;
        let b = a * CalcNode::new_scalar(2.);
        let exp2 = b.exp();
        //let exp2 = (-inp * CalcNodeShared::new_scalar(2.)).exp();
        (&one - &exp2).element_wise_div(&(&one + &exp2))
    }

    pub fn softmax(inp: &CalcNode) -> CalcNode {
        inp.exp().normalized()
    }
}

impl Display for FunctionLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "FunctionLayer ({}): [{}]", self._formula, self._label)
    }
}

impl Parameters for FunctionLayer {
    fn param_iter(&self) -> Box<dyn Iterator<Item = &CalcNode> + '_> {
        Box::new(empty())
    }

    fn param_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut CalcNode> + '_> {
        Box::new(empty())
    }
}

impl Layer for FunctionLayer {
    fn forward(&self, inp: &CalcNode) -> CalcNode {
        (self._func)(inp)
    }
    fn layer_name(&self) -> &str {
        &self._label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn unbiased_layer_forward() {
        let layer = LinearLayer::from_nodes(
            CalcNode::filled_from_shape((2, 2), vec![1., 2., 3., 4.]),
            None,
            "TestLayer",
        );
        let inp = CalcNode::new_col_vector(vec![5., 6.]);
        let expected_value = &[17., 39.];
        let expected_derivative1 = &[5., 6., 5., 6.];
        let expected_derivative2 = &[4., 6.];
        let mut out = layer.forward(&inp);
        assert_eq!(out.copy_vals(), expected_value);
        out.back_propagation();
        assert_eq!(out.copy_grad(), &[1., 1.]);
        assert_eq!(
            layer
                .param_iter()
                .flat_map(|p| p.copy_grad())
                .collect::<Vec<_>>(),
            expected_derivative1
        );
        assert_eq!(inp.copy_grad(), expected_derivative2);
    }

    #[test]
    fn biased_layer_forward() {
        let layer = LinearLayer::from_nodes(
            CalcNode::filled_from_shape((2, 2), vec![1., 2., 3., 4.]),
            Some(CalcNode::new_col_vector(vec![7., 8.])),
            "TestLayer",
        );
        let inp = CalcNode::new_col_vector(vec![5., 6.]);
        let expected_value = &[17. + 7., 39. + 8.];
        let expected_derivative1 = &[vec![5., 6., 5., 6.], vec![1., 1.]];
        let expected_derivative2 = &[4., 6.];
        let mut out = layer.forward(&inp);
        assert_eq!(out.copy_vals(), expected_value);
        out.back_propagation();
        assert_eq!(out.copy_grad(), &[1., 1.]);
        assert_eq!(
            layer
                .param_iter()
                .map(|p| p.copy_grad())
                .collect::<Vec<_>>(),
            expected_derivative1
        );
        assert_eq!(inp.copy_grad(), expected_derivative2);
    }

    #[test]
    fn tanh_forward() {
        let layer = FunctionLayer::new(&FunctionLayer::tanh, "tanh", "TestLayer");
        let inp = CalcNode::new_col_vector(vec![-10., -2., -1., 0., 1., 2., 10.]);
        let expected_value = &[-1., -0.9640276, -0.7615942, 0., 0.7615942, 0.9640276, 1.];
        let out = layer.forward(&inp);
        for (value, expected_value) in out.copy_vals().iter().zip(expected_value) {
            assert_approx_eq!(value, expected_value);
        }
    }
}