fn main() {
    println!("cargo::rerun-if-changed=./tailwind.css");
    println!("cargo::rerun-if-changed=./tailwind.config.js");
    // println!("cargo::rerun-if-changed=./src");
    println!("cargo::rerun-if-changed=./templates");
    std::process::Command::new("tailwindcss")
        .args([
            "-i",
            "./tailwind.css",
            "-o",
            "./assets/tailwind.css",
            // "--minify",
        ])
        .status()
        .expect("Failed to run TailwindCSS build");
}
