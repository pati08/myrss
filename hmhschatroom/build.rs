fn main() {
    println!("cargo::rerun-if-changed=./tailwind.css");
    println!("cargo::rerun-if-changed=./tailwind.config.js");
    std::process::Command::new("tailwindcss")
        .args([
            "-i",
            "./src/tailwind.css",
            "-o",
            "./templates/tailwind.css",
            "--minify",
        ])
        .status()
        .expect("Failed to run TailwindCSS build");
}
