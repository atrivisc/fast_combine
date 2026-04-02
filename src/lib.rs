#[pyo3::pymodule]
mod fast_combine {
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyMapping, PyString, PyList, PyBytes, PyByteArray};

    #[pyfunction]
    pub fn merge_dicts<'py>(
        py: Python<'py>,
        dict_a: &Bound<'py, PyMapping>,
        dict_b: &Bound<'py, PyMapping>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let result = PyDict::new(py);

        result.update(dict_a)?;

        for key in dict_b.keys()? {
            let val_b = dict_b.get_item(&key)?;
            match result.get_item(&key)? {
                Some(val_a) => {
                    if is_mergeable_mapping(val_a.as_any()) && is_mergeable_mapping(val_b.as_any()) {
                        let sub_a = val_a.cast_into::<PyMapping>()?;
                        let sub_b = val_b.cast_into::<PyMapping>()?;
                        let merged = merge_dicts(py, &sub_a, &sub_b)?;
                        result.set_item(&key, merged)?;
                    } else {
                        result.set_item(&key, val_b)?;
                    }
                }
                None => {
                    result.set_item(&key, val_b)?;
                }
            }
        }
        Ok(result)
    }

    fn is_mergeable_mapping(val: &Bound<'_, PyAny>) -> bool {
        !val.is_instance_of::<PyString>()
            && !val.is_instance_of::<PyBytes>()
            && !val.is_instance_of::<PyByteArray>()
            && !val.is_instance_of::<PyList>()
            && val.cast::<PyMapping>().is_ok()
    }

    #[pyfunction]
    pub fn merge_dicts_into<'py>(
        py: Python<'py>,
        dict_a: &Bound<'py, PyMapping>,
        dict_b: &Bound<'py, PyMapping>,
    ) -> PyResult<()> {
        for key in dict_b.keys()? {
            let val_b = dict_b.get_item(&key)?;
            match dict_a.get_item(&key) {
                Ok(val_a) => {
                    if is_mergeable_mapping(val_a.as_any()) && is_mergeable_mapping(val_b.as_any()) {
                        let sub_a = val_a.cast_into::<PyMapping>()?;
                        let sub_b = val_b.cast_into::<PyMapping>()?;
                        merge_dicts_into(py, &sub_a, &sub_b)?;
                    } else {
                        dict_a.set_item(&key, val_b)?;
                    }
                }
                Err(_) => {
                    dict_a.set_item(&key, val_b)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pyo3::prelude::*;
    use pyo3::types::PyDict;
    use crate::fast_combine::{merge_dicts, merge_dicts_into};

    fn setup() {
        Python::initialize();
    }

    fn make_dict<'py>(py: Python<'py>, pairs: &[(&str, i32)]) -> Bound<'py, PyDict> {
        let d = PyDict::new(py);
        for (k, v) in pairs {
            d.set_item(k, v).unwrap();
        }
        d
    }

    #[test]
    fn test_merge_dicts_simple() {
        setup();
        Python::attach(|py| {
            let a = make_dict(py, &[("x", 1)]);
            let b = make_dict(py, &[("y", 2)]);
            let result = merge_dicts(py, a.as_mapping(), b.as_mapping()).unwrap();
            assert_eq!(result.len(), 2);
            assert_eq!(result.get_item("x").unwrap().unwrap().extract::<i32>().unwrap(), 1);
            assert_eq!(result.get_item("y").unwrap().unwrap().extract::<i32>().unwrap(), 2);
        });
    }

    #[test]
    fn test_merge_dicts_overwrite_scalar() {
        setup();
        Python::attach(|py| {
            let a = make_dict(py, &[("x", 1)]);
            let b = make_dict(py, &[("x", 99)]);
            let result = merge_dicts(py, a.as_mapping(), b.as_mapping()).unwrap();
            assert_eq!(result.get_item("x").unwrap().unwrap().extract::<i32>().unwrap(), 99);
        });
    }

    #[test]
    fn test_merge_dicts_does_not_mutate_inputs() {
        setup();
        Python::attach(|py| {
            let a = make_dict(py, &[("x", 1)]);
            let b = make_dict(py, &[("y", 2)]);
            merge_dicts(py, a.as_mapping(), b.as_mapping()).unwrap();
            assert_eq!(a.len(), 1);
            assert_eq!(b.len(), 1);
        });
    }

    #[test]
    fn test_merge_dicts_nested() {
        setup();
        Python::attach(|py| {
            let a = PyDict::new(py);
            let a_inner = make_dict(py, &[("x", 1), ("keep", 2)]);
            a.set_item("nested", &a_inner).unwrap();

            let b = PyDict::new(py);
            let b_inner = make_dict(py, &[("x", 99)]);
            b.set_item("nested", &b_inner).unwrap();

            let result = merge_dicts(py, a.as_mapping(), b.as_mapping()).unwrap();
            let result_inner = result.get_item("nested").unwrap().unwrap();
            let result_inner = result_inner.cast::<PyDict>().unwrap();
            assert_eq!(result_inner.get_item("x").unwrap().unwrap().extract::<i32>().unwrap(), 99);
            assert_eq!(result_inner.get_item("keep").unwrap().unwrap().extract::<i32>().unwrap(), 2);
        });
    }

    #[test]
    fn test_merge_dicts_into_simple() {
        setup();
        Python::attach(|py| {
            let a = make_dict(py, &[("x", 1)]);
            let b = make_dict(py, &[("y", 2)]);
            merge_dicts_into(py, a.as_mapping(), b.as_mapping()).unwrap();
            assert_eq!(a.len(), 2);
            assert_eq!(a.get_item("x").unwrap().unwrap().extract::<i32>().unwrap(), 1);
            assert_eq!(a.get_item("y").unwrap().unwrap().extract::<i32>().unwrap(), 2);
        });
    }

    #[test]
    fn test_merge_dicts_into_overwrite_scalar() {
        setup();
        Python::attach(|py| {
            let a = make_dict(py, &[("x", 1)]);
            let b = make_dict(py, &[("x", 99)]);
            merge_dicts_into(py, a.as_mapping(), b.as_mapping()).unwrap();
            assert_eq!(a.get_item("x").unwrap().unwrap().extract::<i32>().unwrap(), 99);
        });
    }

    #[test]
    fn test_merge_dicts_into_nested_preserves_keys() {
        setup();
        Python::attach(|py| {
            let a = PyDict::new(py);
            let a_inner = make_dict(py, &[("x", 1), ("keep", 2)]);
            a.set_item("nested", &a_inner).unwrap();

            let b = PyDict::new(py);
            let b_inner = make_dict(py, &[("x", 99)]);
            b.set_item("nested", &b_inner).unwrap();

            merge_dicts_into(py, a.as_mapping(), b.as_mapping()).unwrap();

            let a_inner_after = a.get_item("nested").unwrap().unwrap();
            let a_inner_after = a_inner_after.cast::<PyDict>().unwrap();
            assert_eq!(a_inner_after.get_item("x").unwrap().unwrap().extract::<i32>().unwrap(), 99);
            assert_eq!(a_inner_after.get_item("keep").unwrap().unwrap().extract::<i32>().unwrap(), 2);
        });
    }

    #[test]
    fn test_merge_dicts_into_b_not_mutated() {
        setup();
        Python::attach(|py| {
            let a = make_dict(py, &[("x", 1)]);
            let b = make_dict(py, &[("y", 2)]);
            merge_dicts_into(py, a.as_mapping(), b.as_mapping()).unwrap();
            assert_eq!(b.len(), 1);
        });
    }

    #[test]
    fn test_merge_dicts_deeply_nested() {
        setup();
        Python::attach(|py| {
            let a = PyDict::new(py);
            let a_l2 = PyDict::new(py);
            let a_l3 = PyDict::new(py);
            a_l3.set_item("key3", "value").unwrap();
            a_l2.set_item("key2", &a_l3).unwrap();
            a.set_item("key", &a_l2).unwrap();

            let b = PyDict::new(py);
            let b_l2 = PyDict::new(py);
            let b_l3 = PyDict::new(py);
            b_l3.set_item("key4", "value").unwrap();
            b_l2.set_item("key2", &b_l3).unwrap();
            b.set_item("key", &b_l2).unwrap();

            let result = merge_dicts(py, a.as_mapping(), b.as_mapping()).unwrap();

            let l2 = result.get_item("key").unwrap().unwrap();
            let l2 = l2.cast::<PyDict>().unwrap();
            let l3 = l2.get_item("key2").unwrap().unwrap();
            let l3 = l3.cast::<PyDict>().unwrap();

            assert_eq!(l3.get_item("key3").unwrap().unwrap().extract::<&str>().unwrap(), "value");
            assert_eq!(l3.get_item("key4").unwrap().unwrap().extract::<&str>().unwrap(), "value");
            assert_eq!(l3.len(), 2);
        });
    }

    #[test]
    fn test_merge_dicts_into_deeply_nested() {
        setup();
        Python::attach(|py| {
            let a = PyDict::new(py);
            let a_l2 = PyDict::new(py);
            let a_l3 = PyDict::new(py);
            a_l3.set_item("key3", "value").unwrap();
            a_l2.set_item("key2", &a_l3).unwrap();
            a.set_item("key", &a_l2).unwrap();

            let b = PyDict::new(py);
            let b_l2 = PyDict::new(py);
            let b_l3 = PyDict::new(py);
            b_l3.set_item("key4", "value").unwrap();
            b_l2.set_item("key2", &b_l3).unwrap();
            b.set_item("key", &b_l2).unwrap();

            merge_dicts_into(py, a.as_mapping(), b.as_mapping()).unwrap();

            let l2 = a.get_item("key").unwrap().unwrap();
            let l2 = l2.cast::<PyDict>().unwrap();
            let l3 = l2.get_item("key2").unwrap().unwrap();
            let l3 = l3.cast::<PyDict>().unwrap();

            assert_eq!(l3.get_item("key3").unwrap().unwrap().extract::<&str>().unwrap(), "value");
            assert_eq!(l3.get_item("key4").unwrap().unwrap().extract::<&str>().unwrap(), "value");
            assert_eq!(l3.len(), 2);
        });
    }
}
