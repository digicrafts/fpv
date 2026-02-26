import React, { useMemo, useState } from "react";

type User = {
  id: number;
  name: string;
  role: "admin" | "viewer";
};

const users: User[] = [
  { id: 1, name: "Ada", role: "admin" },
  { id: 2, name: "Lin", role: "viewer" },
];

export default function UserList(): JSX.Element {
  const [query, setQuery] = useState("");

  const filtered = useMemo(
    () => users.filter((u) => u.name.toLowerCase().includes(query.toLowerCase())),
    [query],
  );

  return (
    <section className="panel">
      <h1>Users</h1>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search by name"
      />
      <ul>
        {filtered.map((u) => (
          <li key={u.id} data-role={u.role}>
            <strong>{u.name}</strong> <em>({u.role})</em>
          </li>
        ))}
      </ul>
    </section>
  );
}
