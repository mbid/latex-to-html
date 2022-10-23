# latex-to-html

latex-to-html converts a subset of latex to html.
It renders formulas as vector graphics, so that they look exactly as they would in a pdf.  
[**Example**](https://latex-to-html.mbid.me/)

## Features

Because latex-to-html renders math formulas using pdflatex, it supports the same math formulas as latex.
Inline math must be delimited by `$ ... $` and display math must be of the form
```
\begin{equation}
  \label{....}
  ...
\end{equation}
```
where the label is optional.
Outside of math environments, only a subset of tex/latex is supported:

- `\title{...}`
- `\section{...}`
- `\subsection{...}`
- `\begin{itemize} \item ... \item ... \end{itemize}`
- `\begin{enumerate} \item ... \item ... \end{enumerate}`
- Some hard-coded theorem-like environments (those usually declared with `\newtheorem`):
  * `theorem`
  * `proposition`
  * `definition`
  * `lemma`
  * `remark`
  * `corollary`
  * `example`
- `\begin{proof} ... \end{proof}`
- `\label{...}`, `\ref{...}` and `\eqref{...}`
- `\emph{...}`, `\textbf{...}`, `\textit{...}`

There is also basic support for `\bibliography`.

Latex-to-html ignores the lines directly after a line containing the following comment:
```
% LATEX_TO_HTML_IGNORE
```
This is useful in case a tex file is also used with pdflatex directly to generate a pdf file.

For example, latex-to-html does not support the `enumitem` package.
To use it for the generated pdf output anyway, the package can be included as follows:
```
% LATEX_TO_HTML_IGNORE
\usepackage{enumitem}
```
The appearance of numerals in an enumeration for the pdf output (but not the webpage) can then be changed like so:
```
\begin{enumerate}
  % LATEX_TO_HTML_IGNORE
  [label={(\roman*)}]
  \item
    ...
  \item
    ...
\end{enumerate}
```

## Installation

Make sure to install all dependencies first.
latex-to-html depends on cargo, pdflatex, pdfcrop and pdf2svg.
On Debian or Ubuntu, these dependencies can be installed as follows:
```
sudo apt install cargo texlive texlive-extra-utils pdf2svg
```

To download and install latex-to-html to `~/.cargo/bin`, run the following:
```
cargo install latex-to-html
```
You can now either add `$HOME/.cargo/bin` to your `PATH` variable or simply specify the full path when you execute latex-to-html: `~/.cargo/bin/latex-to-html`.

## Usage

Assuming you have a `doc.tex` and a `doc.bib` file, run the following:
```
latex-to-html doc.tex doc.bib out/
```
This may take a while on the first run, but subsequent runs will be much faster.
To view the generated document, open `out/index.html` in your browser.
