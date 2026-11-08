export async function getServerSideProps() {
  return { props: { now: Date.now() } };
}

export default function Legacy({ now }: { now: number }) {
  return <p>{now}</p>;
}
