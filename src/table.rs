use crate::{context::ContentHierarchy, error::TableExtractorError, text::RichText};
use hashbrown::HashMap;
use pyo3::prelude::*;

#[derive(Debug, Clone)]
#[pyclass]
pub struct Table {
    pub id: String,
    pub url: String,
    pub caption: String,
    pub attrs: HashMap<String, String>,
    pub context: Vec<ContentHierarchy>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct Row {
    pub cells: Vec<Cell>,
    pub attrs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct Cell {
    pub is_header: bool,
    pub rowspan: u16,
    pub colspan: u16,
    pub attrs: HashMap<String, String>,
    pub value: RichText,
    // raw html of the cell
    pub html: String,
}

impl Table {
    /// Span the table by copying values to merged field
    pub fn span(&self) -> Result<Table, TableExtractorError> {
        if self.rows.len() == 0 {
            return Ok(self.clone());
        }

        let mut pi = 0;
        let mut data = vec![];
        let mut pending_ops = HashMap::<(i32, i32), Cell>::new();

        // >>> begin find the max #cols
        // calculate the number of columns as some people may actually set unrealistic colspan as they are lazy..
        // I try to make its behaviour as much closer to the browser as possible.
        // one thing I notice that to find the correct value of colspan, they takes into account the #cells of rows below the current row
        // so we may have to iterate several times

        let mut cols = vec![0; self.rows.len()];
        for (i, row) in self.rows.iter().enumerate() {
            cols[i] += row.cells.len();
            for cell in &row.cells {
                if cell.rowspan > 1 {
                    for j in 1..cell.rowspan {
                        if i + (j as usize) < cols.len() {
                            cols[i + (j as usize)] += 1;
                        }
                    }
                }
            }
        }

        let max_ncols = *cols.iter().enumerate().max_by_key(|x| x.1).unwrap().1 as i32;

        // sometimes they do show an extra cell for over-colspan row, but it's not consistent or at least not easy for me to find the rule
        // so I decide to not handle that. Hope that we don't have many tables like that.
        // >>> finish find the max #cols

        for row in &self.rows {
            let mut new_row = Vec::with_capacity(row.cells.len());
            let mut pj = 0;

            for (cell_index, ocell) in row.cells.iter().enumerate() {
                let mut cell = ocell.clone();
                cell.colspan = 1;
                cell.rowspan = 1;

                // adding cell from the top
                while pending_ops.contains_key(&(pi, pj)) {
                    new_row.push(pending_ops.remove(&(pi, pj)).unwrap());
                    pj += 1;
                }

                // now add cell and expand the column
                for _ in 0..ocell.colspan {
                    if pending_ops.contains_key(&(pi, pj)) {
                        // exception, overlapping between colspan and rowspan
                        return Err(TableExtractorError::OverlapSpanError("".to_owned()).into());
                    }
                    new_row.push(cell.clone());
                    for ioffset in 1..ocell.rowspan {
                        pending_ops.insert((pi + ioffset as i32, pj), cell.clone());
                    }
                    pj += 1;

                    if pj >= max_ncols {
                        // our algorithm cannot handle the case where people are bullying the colspan system, and only can handle the case
                        // where the span that goes beyond the maximum number of columns is in the last column.
                        if cell_index != row.cells.len() - 1 {
                            return Err(
                                TableExtractorError::InvalidCellSpanError("".to_owned()).into()
                            );
                        } else {
                            break;
                        }
                    }
                }
            }

            // add more cells from the top since we reach the end
            while pending_ops.contains_key(&(pj, pj)) && pj < max_ncols {
                new_row.push(pending_ops.remove(&(pj, pj)).unwrap());
                pj += 1;
            }

            data.push(Row {
                cells: new_row,
                attrs: row.attrs.clone(),
            });
            pi += 1;
        }

        // len(pending_ops) may > 0, but fortunately, it doesn't matter as the browser also does not render that extra empty lines

        Ok(Table {
            id: self.id.clone(),
            url: self.url.clone(),
            caption: self.caption.clone(),
            attrs: self.attrs.clone(),
            context: self.context.clone(),
            rows: data,
        })
    }

    /// Pad an irregular table (missing cells) to make it become a regular table
    ///
    /// This function only return new table when it's padded, otherwise, None.
    pub fn pad(&self) -> Option<Table> {
        if self.rows.len() == 0 {
            return None;
        }

        let ncols = self.rows[0].cells.len();
        let is_regular_table = self.rows.iter().all(|row| row.cells.len() == ncols);
        if is_regular_table {
            return None;
        }

        let max_ncols = self.rows.iter().map(|row| row.cells.len()).max().unwrap();
        let default_cell = Cell {
            is_header: false,
            rowspan: 1,
            colspan: 1,
            attrs: HashMap::new(),
            value: RichText::empty(),
            html: "".to_owned(),
        };

        let mut rows = Vec::with_capacity(self.rows.len());
        for r in &self.rows {
            let mut row = r.clone();

            let mut newcell = default_cell.clone();
            // heuristic to match header from the previous cell of the same row
            newcell.is_header = row.cells.last().map_or(false, |cell| cell.is_header);

            while row.cells.len() < max_ncols {
                row.cells.push(newcell.clone());
            }
            rows.push(row);
        }

        Some(Table {
            id: self.id.clone(),
            url: self.url.clone(),
            caption: self.caption.clone(),
            attrs: self.attrs.clone(),
            context: self.context.clone(),
            rows: rows,
        })
    }
}