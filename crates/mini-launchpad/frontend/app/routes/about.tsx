import { Link } from "@remix-run/react";

export default function About() {
  return (
    <main className="page">
      <section className="hero">
        <h1>О проекте</h1>
        <p className="lead">
          Mini Launchpad: oracle SOL/USD + token minter. Backend обновляет цену и слушает события.
        </p>
        <Link className="button ghost" to="/">
          Назад на главную
        </Link>
      </section>
    </main>
  );
}
